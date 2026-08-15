mod app;
mod cache;
mod config;
mod database;
mod execution;
mod logging;
mod policy;
mod query;
mod shutdown;
mod squeal;
mod value;

use std::process::ExitCode;
use std::sync::Arc;

use thiserror::Error;
use tracing::info;

use app::AppState;
use cache::SelectCache;
use config::Config;

#[tokio::main]
async fn main() -> ExitCode {
    logging::init();

    if let Err(error) = run().await {
        tracing::error!(%error, "server failed to start");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

async fn run() -> Result<(), StartupError> {
    let config_path = config::path_from_args()?;
    let config = Config::load(&config_path)?;
    let databases = database::open_all(&config).await?;
    let cache = Arc::new(SelectCache::new(config.cache_max_entries()));
    let state = AppState { databases, cache };
    let application = app::router(state, &config);
    let listener = tokio::net::TcpListener::bind(config.bind_address).await?;

    let database_names: Vec<&str> = config
        .databases()
        .iter()
        .map(|database| database.name.as_str())
        .collect();

    info!(
        config_path = %config_path.display(),
        address = %config.bind_address,
        databases = ?database_names,
        request_body_limit_bytes = config.request_body_limit_bytes(),
        request_timeout_seconds = config.request_timeout().as_secs(),
        long_poll_timeout_seconds = config.long_poll_timeout().as_secs(),
        cache_max_entries = config.cache_max_entries(),
        "server listening"
    );

    axum::serve(listener, application)
        .with_graceful_shutdown(shutdown::signal())
        .await
        .expect("axum server is infallible");

    Ok(())
}

#[derive(Debug, Error)]
enum StartupError {
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error(transparent)]
    Database(#[from] database::OpenError),
    #[error("could not bind HTTP listener: {0}")]
    Listener(#[from] std::io::Error),
}
