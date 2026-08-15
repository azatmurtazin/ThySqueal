mod app;
mod cache;
mod config;
mod database;
mod events;
mod execution;
mod logging;
mod policy;
mod query;
mod shutdown;
mod squeal;
mod value;

use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::watch;
use tracing::info;

use app::AppState;
use config::Config;
use events::WaiterLimits;

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

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let (signal_tx, signal_rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        shutdown::signal().await;
        let _ = shutdown_tx.send(true);
        let _ = signal_tx.send(());
    });

    let state = AppState {
        databases,
        waiters: Arc::new(WaiterLimits::new(
            config.long_poll_max_waiters(),
            config.long_poll_max_waiters_per_client(),
        )),
        shutdown: shutdown_rx,
        long_poll_timeout: config.long_poll_timeout(),
    };
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
        long_poll_max_waiters = config.long_poll_max_waiters(),
        cache_max_entries = config.cache_max_entries(),
        "server listening"
    );

    axum::serve(
        listener,
        application.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = signal_rx.await;
    })
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
