use axum::{
    debug_handler,
    routing::{get, put},
    Router,
};
use color_eyre::Result;
use serde::Serialize;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        OnceLock,
    },
};
use tokio::fs::read_to_string;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

static COUNTER: OnceLock<Counter> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();

    let counter = Counter::read_from_path(Path::new("./save")).await?;
    COUNTER
        .set(counter)
        .expect("You're really fucked up if even this fails");

    let app = Router::new()
        .route("/api/v1/counter", get(get_counter_val))
        .route("/api/v1/counter/increment", put(increment_counter))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    axum::Server::bind(&"0.0.0.0:1066".parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

#[debug_handler]
async fn get_counter_val() -> String {
    let counter = COUNTER.get().expect("COUNTER is initialized");
    let val = counter.get_val();
    val.to_string()
}

#[debug_handler]
async fn increment_counter() {
    let counter = COUNTER.get().expect("COUNTER is initialized");
    counter.increment();
}

#[derive(Debug, Serialize)]
struct Counter(AtomicUsize);

impl Counter {
    async fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Counter> {
        let save = read_to_string(path).await?;
        // this is a very stupid thing
        let save = save.trim_end_matches('\n');
        let counter: usize = dbg!(save).parse()?;

        Ok(Counter(AtomicUsize::new(counter)))
    }

    fn get_val(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }

    fn increment(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
