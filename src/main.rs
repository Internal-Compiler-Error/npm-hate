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
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::prelude::*;
use tracing::{instrument, info};


static COUNTER: OnceLock<Counter> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let tracing_layer = tracing_subscriber::fmt::layer().pretty();
    tracing_subscriber::registry()
        .with(tracing_layer)
        .init();


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
        .with_graceful_shutdown(shutdown_signal())
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

#[tracing::instrument]
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("signal received, starting graceful shutdown");

    info!("saving counter value to file");
    let counter = COUNTER.get().expect("COUNTER is initialized");
    let val = counter.get_val();
    tokio::fs::write("./save", val.to_string())
        .await
        .expect("Failed to write to save file");

    info!("bye");
}

#[derive(Debug, Serialize)]
struct Counter(AtomicUsize);

impl Counter {
    async fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Counter> {
        let save = read_to_string(path).await?;
        // this is a very stupid thing
        let save = save.trim_end_matches('\n');
        let counter: usize = save.parse()?;

        Ok(Counter(AtomicUsize::new(counter)))
    }

    fn get_val(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }

    fn increment(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
