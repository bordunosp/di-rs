```rust

struct CounterService {
    counter: i32,
}

impl CounterService {
    pub fn increment(&mut self) -> i32 {
        self.counter += 1;
        self.counter
    }
}

struct CounterService2 {
    counter: i32,
}

impl CounterService2 {
    pub fn increment(&mut self) -> i32 {
        self.counter += 1;
        self.counter
    }
}

struct CounterService3 {
    counter: i32,
}

impl CounterService3 {
    pub fn increment(&mut self) -> i32 {
        self.counter += 1;
        self.counter
    }
}


#[tokio::main]
async fn main() -> Result<(), DiError> {
    di_rs::register_singleton(CounterService { counter: 0 }).await?;
    di_rs::register_scope(|| async {CounterService3 { counter: 0 }}).await?;
    di_rs::register_transient(|| async { CounterService2 { counter: 0 } }).await?;

    di_rs::register_singleton_name("CounterService", CounterService { counter: 0 }).await?;
    di_rs::register_scope_name("CounterService3", || async {CounterService3 { counter: 0 }}).await?;
    di_rs::register_transient_name("CounterService2", || async {CounterService2 { counter: 0 }}).await?;

    let di = di_rs::scope();

    match di.get::<CounterService>().await {
        Ok(service) => {
            let mut locked_service = service.write().await;
            println!("singleton: {}", locked_service.increment());
            println!("singleton: {}", locked_service.increment());
            drop(locked_service);
        }
        Err(e) => println!("Error: {e}"),
    }

    match di.get::<CounterService>().await {
        Ok(service) => {
            let mut locked_service = service.write().await;
            println!("singleton: {}", locked_service.increment());
        }
        Err(e) => println!("Error: {e}"),
    }

    match di.by_name::<CounterService>("CounterService").await {
        Ok(service) => {
            let mut locked_service = service.write().await;
            println!("singleton named: {}", locked_service.increment());
            println!("singleton named: {}", locked_service.increment());
            drop(locked_service);
        }
        Err(e) => println!("Error: {e}"),
    }

    match di.by_name::<CounterService>("CounterService").await {
        Ok(service) => {
            let mut locked_service = service.write().await;
            println!("singleton named: {}", locked_service.increment());
        }
        Err(e) => println!("Error: {e}"),
    }

    match di.get::<CounterService2>().await {
        Ok(service) => {
            let mut locked = service.write().await;
            println!("Transient 1: {}", locked.increment());
        }
        Err(e) => println!("Error: {e}"),
    }

    match di.get::<CounterService2>().await {
        Ok(service) => {
            let mut locked = service.write().await;
            println!("Transient 2: {}", locked.increment());
        }
        Err(e) => println!("Error: {e}"),
    }

    match di.by_name::<CounterService2>("CounterService2").await {
        Ok(service) => {
            let mut locked = service.write().await;
            println!("Transient named 1: {}", locked.increment());
        }
        Err(e) => println!("Error: {e}"),
    }

    match di.by_name::<CounterService2>("CounterService2").await {
        Ok(service) => {
            let mut locked = service.write().await;
            println!("Transient named 2: {}", locked.increment());
        }
        Err(e) => println!("Error: {e}"),
    }

    match di.get::<CounterService3>().await {
        Ok(service) => {
            let mut locked = service.write().await;
            println!("Scope 1: {}", locked.increment());
        }
        Err(e) => println!("Error: {e}"),
    }

    match di.get::<CounterService3>().await {
        Ok(service) => {
            let mut locked = service.write().await;
            println!("Scope 2: {}", locked.increment());
        }
        Err(e) => println!("Error: {e}"),
    }

    match di.by_name::<CounterService3>("CounterService3").await {
        Ok(service) => {
            let mut locked = service.write().await;
            println!("Scope named 1: {}", locked.increment());
        }
        Err(e) => println!("Error: {e}"),
    }

    match di.by_name::<CounterService3>("CounterService3").await {
        Ok(service) => {
            let mut locked = service.write().await;
            println!("Scope named 2: {}", locked.increment());
        }
        Err(e) => println!("Error: {e}"),
    }

    Ok(())
}
```