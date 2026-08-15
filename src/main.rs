mod app;
mod config;
mod database;
mod logging;
mod shutdown;

use std::process::ExitCode;

use thiserror::Error;
use tracing::info;

use app::AppState;
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
    let database = database::open(&config).await?;
    let state = AppState { database };
    let application = app::router(state, &config);
    let listener = tokio::net::TcpListener::bind(config.bind_address).await?;

    info!(
        config_path = %config_path.display(),
        address = %config.bind_address,
        database_path = %config.database_path().display(),
        max_connections = config.database_max_connections(),
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
    #[error("could not open SQLite database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("could not bind HTTP listener: {0}")]
    Listener(#[from] std::io::Error),
}
