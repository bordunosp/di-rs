use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Debug, Error)]
pub enum DiError {
    #[error("Service not found for type {0} with name '{1}'")]
    ServiceNotFound(&'static str, String),

    #[error("Service already registered for type {0} with name '{1}'")]
    ServiceAlreadyRegistered(&'static str, String),

    #[error("Failed to downcast service for type {0} with name '{1}'")]
    DowncastFailed(&'static str, String),
}

#[async_trait]
trait FactoryFunction: Send + Sync {
    async fn create(&self) -> Arc<dyn Any + Send + Sync>;
}

struct FactoryFn<F, Fut, T> {
    factory: F,
    _phantom: std::marker::PhantomData<(Fut, T)>,
}

#[async_trait]
impl<F, Fut, T> FactoryFunction for FactoryFn<F, Fut, T>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + Sync + 'static,
    T: Any + Send + Sync + 'static,
{
    async fn create(&self) -> Arc<dyn Any + Send + Sync> {
        let result = (self.factory)().await;
        Arc::new(RwLock::new(result)) as Arc<dyn Any + Send + Sync>
    }
}

type ServiceKey = (TypeId, String);
type SingletonServices = Arc<RwLock<HashMap<ServiceKey, Arc<dyn Any + Send + Sync>>>>;
type TransientServices = Arc<RwLock<HashMap<ServiceKey, Arc<dyn FactoryFunction>>>>;
type ScopeServices = Arc<RwLock<HashMap<ServiceKey, Arc<dyn FactoryFunction>>>>;

lazy_static::lazy_static! {
    static ref REGISTERED_SERVICES_SINGLETON: SingletonServices = Arc::new(RwLock::new(HashMap::new()));
    static ref REGISTERED_SERVICES_TRANSIENT: TransientServices = Arc::new(RwLock::new(HashMap::new()));
    static ref REGISTERED_SERVICES_SCOPE: ScopeServices = Arc::new(RwLock::new(HashMap::new()));
}

pub struct DI {
    services: SingletonServices,
}
impl DI {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn dispose(&self) {
        let mut registry = REGISTERED_SERVICES_SCOPE.write().await;
        registry.clear();
    }

    pub async fn get<T>(&self) -> Result<Arc<RwLock<T>>, DiError>
    where
        T: Any + Send + Sync + 'static,
    {
        self.by_name("").await
    }

    pub async fn by_name<T>(&self, name: &str) -> Result<Arc<RwLock<T>>, DiError>
    where
        T: Any + Send + Sync + 'static,
    {
        if let Ok(service) = get_singleton::<T>(name).await {
            return Ok(service);
        }

        if let Ok(service) = get_transient::<T>(name).await {
            return Ok(service);
        }

        if let Ok(service) = self.get_services::<T>(name).await {
            return Ok(service);
        } else {
            if let Ok(service) = get_scope::<T>(name).await {
                {
                    let key = (TypeId::of::<T>(), name.to_string());
                    self.services.write().await.insert(key, service);
                }
            }
            if let Ok(service) = self.get_services::<T>(name).await {
                return Ok(service);
            }
        }

        Err(DiError::ServiceNotFound(
            std::any::type_name::<T>(),
            name.to_string(),
        ))
    }

    async fn get_services<T: Any + Send + Sync + 'static>(
        &self,
        name: &str,
    ) -> Result<Arc<RwLock<T>>, DiError> {
        let type_name = std::any::type_name::<T>();
        let key = (TypeId::of::<T>(), name.to_string());

        let registry = self.services.read().await;
        let service_any = registry
            .get(&key)
            .ok_or(DiError::ServiceNotFound(type_name, name.to_string()))?;

        service_any
            .clone()
            .downcast::<RwLock<T>>()
            .map_err(|_| DiError::DowncastFailed(type_name, name.to_string()))
    }
}

pub fn scope() -> DI {
    DI::new()
}

pub async fn register_transient<F, Fut, T>(factory: F) -> Result<(), DiError>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + Sync + 'static,
    T: Any + Send + Sync + 'static,
{
    register_transient_name("", factory).await
}

pub async fn register_transient_name<F, Fut, T>(name: &str, factory: F) -> Result<(), DiError>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + Sync + 'static,
    T: Any + Send + Sync + 'static,
{
    let type_name = std::any::type_name::<T>();
    let key = (TypeId::of::<T>(), name.to_string());

    let boxed_factory = Arc::new(FactoryFn {
        factory,
        _phantom: std::marker::PhantomData,
    }) as Arc<dyn FactoryFunction>;

    {
        if REGISTERED_SERVICES_SINGLETON
            .read()
            .await
            .contains_key(&key)
        {
            return Err(DiError::ServiceAlreadyRegistered(
                type_name,
                name.to_string(),
            ));
        }
    }
    {
        if REGISTERED_SERVICES_SCOPE.read().await.contains_key(&key) {
            return Err(DiError::ServiceAlreadyRegistered(
                type_name,
                name.to_string(),
            ));
        }
    }

    let mut registry = REGISTERED_SERVICES_TRANSIENT.write().await;

    if registry.contains_key(&key) {
        return Err(DiError::ServiceAlreadyRegistered(
            type_name,
            name.to_string(),
        ));
    }

    registry.insert(key, boxed_factory);

    Ok(())
}

pub async fn register_scope<F, Fut, T>(factory: F) -> Result<(), DiError>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + Sync + 'static,
    T: Any + Send + Sync + 'static,
{
    register_scope_name("", factory).await
}

pub async fn register_scope_name<F, Fut, T>(name: &str, factory: F) -> Result<(), DiError>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = T> + Send + Sync + 'static,
    T: Any + Send + Sync + 'static,
{
    let type_name = std::any::type_name::<T>();
    let key = (TypeId::of::<T>(), name.to_string());

    let boxed_factory = Arc::new(FactoryFn {
        factory,
        _phantom: std::marker::PhantomData,
    }) as Arc<dyn FactoryFunction>;

    {
        if REGISTERED_SERVICES_SINGLETON
            .read()
            .await
            .contains_key(&key)
        {
            return Err(DiError::ServiceAlreadyRegistered(
                type_name,
                name.to_string(),
            ));
        }
    }
    {
        if REGISTERED_SERVICES_TRANSIENT
            .read()
            .await
            .contains_key(&key)
        {
            return Err(DiError::ServiceAlreadyRegistered(
                type_name,
                name.to_string(),
            ));
        }
    }

    let mut registry = REGISTERED_SERVICES_SCOPE.write().await;

    if registry.contains_key(&key) {
        return Err(DiError::ServiceAlreadyRegistered(
            type_name,
            name.to_string(),
        ));
    }

    registry.insert(key, boxed_factory);

    Ok(())
}

pub async fn register_singleton<T: Any + Send + Sync + 'static>(service: T) -> Result<(), DiError> {
    register_singleton_name("", service).await
}

pub async fn register_singleton_name<T: Any + Send + Sync + 'static>(
    name: &str,
    service: T,
) -> Result<(), DiError> {
    let type_name = std::any::type_name::<T>();
    let key = (TypeId::of::<T>(), name.to_string());

    {
        if REGISTERED_SERVICES_SCOPE.read().await.contains_key(&key) {
            return Err(DiError::ServiceAlreadyRegistered(
                type_name,
                name.to_string(),
            ));
        }
    }
    {
        if REGISTERED_SERVICES_TRANSIENT
            .read()
            .await
            .contains_key(&key)
        {
            return Err(DiError::ServiceAlreadyRegistered(
                type_name,
                name.to_string(),
            ));
        }
    }

    let mut registry = REGISTERED_SERVICES_SINGLETON.write().await;

    if registry.contains_key(&key) {
        return Err(DiError::ServiceAlreadyRegistered(
            type_name,
            name.to_string(),
        ));
    }

    let service = Arc::new(RwLock::new(service)) as Arc<dyn Any + Send + Sync>;
    registry.insert(key, service);

    Ok(())
}

async fn get_singleton<T: Any + Send + Sync + 'static>(
    name: &str,
) -> Result<Arc<RwLock<T>>, DiError> {
    let type_name = std::any::type_name::<T>();
    let key = (TypeId::of::<T>(), name.to_string());

    let registry = REGISTERED_SERVICES_SINGLETON.read().await;
    let service_any = registry
        .get(&key)
        .ok_or(DiError::ServiceNotFound(type_name, name.to_string()))?;

    service_any
        .clone()
        .downcast::<RwLock<T>>()
        .map_err(|_| DiError::DowncastFailed(type_name, name.to_string()))
}

async fn get_scope<T>(name: &str) -> Result<Arc<RwLock<T>>, DiError>
where
    T: Any + Send + Sync + 'static,
{
    let type_name = std::any::type_name::<T>();
    let key = (TypeId::of::<T>(), name.to_string());

    let registry = REGISTERED_SERVICES_SCOPE.read().await;
    let factory = registry
        .get(&key)
        .ok_or(DiError::ServiceNotFound(type_name, name.to_string()))?;

    let service = factory.create().await;
    service
        .downcast::<RwLock<T>>()
        .map_err(|_| DiError::DowncastFailed(type_name, name.to_string()))
}

async fn get_transient<T>(name: &str) -> Result<Arc<RwLock<T>>, DiError>
where
    T: Any + Send + Sync + 'static,
{
    let type_name = std::any::type_name::<T>();
    let key = (TypeId::of::<T>(), name.to_string());

    let registry = REGISTERED_SERVICES_TRANSIENT.read().await;
    let factory = registry
        .get(&key)
        .ok_or(DiError::ServiceNotFound(type_name, name.to_string()))?;

    let service = factory.create().await;
    service
        .downcast::<RwLock<T>>()
        .map_err(|_| DiError::DowncastFailed(type_name, name.to_string()))
}
