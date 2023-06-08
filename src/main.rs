use axum::{
    debug_handler,
    routing::{get, put},
    Router,
};
use color_eyre::Result;
use std::{path::Path, sync::OnceLock};
use axum::http::Method;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::prelude::*;
use tower_http::cors::{Any, CorsLayer};

mod counter;
use counter::Counter;

static COUNTER: OnceLock<Counter> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let tracing_layer = tracing_subscriber::fmt::layer().pretty();
    tracing_subscriber::registry().with(tracing_layer).init();

    let counter = Counter::read_from_path(Path::new("./save")).await?;
    COUNTER
        .set(counter)
        .expect("You're really fucked up if even this fails");

    let cors = CorsLayer::new().allow_methods(vec![Method::GET, Method::PUT])
        .allow_origin(Any);

    let app = Router::new()
        .route("/api/v1/status", get(ok))
        .route("/api/v1/counter", get(get_counter_val))
        .route("/api/v1/counter/increment", put(increment_counter))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
        .layer(cors);

    axum::Server::bind(&"127.0.0.1:1066".parse()?)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn ok() -> () {
    ()
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

/// Save our counter to a file when shutdown is called
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
