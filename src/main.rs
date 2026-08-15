use std::{env, net::SocketAddr, path::PathBuf, str::FromStr, time::Duration};

use axum::{
    Router,
    extract::State,
    http::{HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use thiserror::Error;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, filter::LevelFilter};
use uuid::Uuid;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:3000";
const DEFAULT_DATABASE_PATH: &str = "thy-squeal.db";
const DEFAULT_MAX_CONNECTIONS: &str = "5";
const DEFAULT_REQUEST_BODY_LIMIT_BYTES: &str = "1048576";
const DEFAULT_REQUEST_TIMEOUT_SECONDS: &str = "30";
const DEFAULT_LONG_POLL_TIMEOUT_SECONDS: &str = "30";
const DEFAULT_CACHE_MAX_ENTRIES: &str = "1000";

#[derive(Clone)]
struct AppState {
    database: SqlitePool,
}

#[derive(Debug)]
struct Config {
    bind_address: SocketAddr,
    database_path: PathBuf,
    database_max_connections: u32,
    request_body_limit_bytes: usize,
    request_timeout: Duration,
    long_poll_timeout: Duration,
    cache_max_entries: u64,
}

impl Config {
    fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            bind_address: parse_env("THY_SQUEAL_BIND_ADDRESS", DEFAULT_BIND_ADDRESS)?,
            database_path: env::var("THY_SQUEAL_DATABASE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(DEFAULT_DATABASE_PATH)),
            database_max_connections: parse_env(
                "THY_SQUEAL_DATABASE_MAX_CONNECTIONS",
                DEFAULT_MAX_CONNECTIONS,
            )?,
            request_body_limit_bytes: parse_env(
                "THY_SQUEAL_REQUEST_BODY_LIMIT_BYTES",
                DEFAULT_REQUEST_BODY_LIMIT_BYTES,
            )?,
            request_timeout: Duration::from_secs(parse_env(
                "THY_SQUEAL_REQUEST_TIMEOUT_SECONDS",
                DEFAULT_REQUEST_TIMEOUT_SECONDS,
            )?),
            long_poll_timeout: Duration::from_secs(parse_env(
                "THY_SQUEAL_LONG_POLL_TIMEOUT_SECONDS",
                DEFAULT_LONG_POLL_TIMEOUT_SECONDS,
            )?),
            cache_max_entries: parse_env(
                "THY_SQUEAL_CACHE_MAX_ENTRIES",
                DEFAULT_CACHE_MAX_ENTRIES,
            )?,
        })
    }
}

fn parse_env<T>(name: &'static str, default: &'static str) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let value = env::var(name).unwrap_or_else(|_| default.to_owned());
    value
        .parse()
        .map_err(|error: T::Err| ConfigError::InvalidValue {
            name,
            value,
            reason: error.to_string(),
        })
}

#[derive(Debug, Error)]
enum ConfigError {
    #[error("invalid value for {name}: {value} ({reason})")]
    InvalidValue {
        name: &'static str,
        value: String,
        reason: String,
    },
}

#[derive(Debug, Error)]
enum StartupError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("could not open SQLite database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("could not bind HTTP listener: {0}")]
    Listener(#[from] std::io::Error),
}

#[derive(Clone)]
struct MakeRequestUuid;

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        HeaderValue::from_str(&Uuid::new_v4().to_string())
            .ok()
            .map(RequestId::new)
    }
}

#[tokio::main]
async fn main() {
    init_tracing();

    if let Err(error) = run().await {
        error!(%error, "server failed to start");
        std::process::exit(1);
    }
}

fn init_tracing() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

async fn run() -> Result<(), StartupError> {
    let config = Config::from_env()?;
    let database = open_database(&config).await?;
    let state = AppState { database };
    let application = app(state, &config);
    let listener = tokio::net::TcpListener::bind(config.bind_address).await?;

    info!(
        address = %config.bind_address,
        database_path = %config.database_path.display(),
        max_connections = config.database_max_connections,
        request_body_limit_bytes = config.request_body_limit_bytes,
        request_timeout_seconds = config.request_timeout.as_secs(),
        long_poll_timeout_seconds = config.long_poll_timeout.as_secs(),
        cache_max_entries = config.cache_max_entries,
        "server listening"
    );

    axum::serve(listener, application)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("axum server is infallible");

    Ok(())
}

async fn open_database(config: &Config) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(&config.database_path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    SqlitePoolOptions::new()
        .max_connections(config.database_max_connections)
        .connect_with(options)
        .await
}

fn app(state: AppState, config: &Config) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(readiness))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(CatchPanicLayer::new())
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(PropagateRequestIdLayer::x_request_id())
                .layer(RequestBodyLimitLayer::new(config.request_body_limit_bytes))
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    config.request_timeout,
                )),
        )
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn readiness(State(state): State<AppState>) -> Result<StatusCode, ReadinessError> {
    sqlx::query("SELECT 1").execute(&state.database).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Error)]
#[error("database is unavailable")]
struct ReadinessError(#[source] sqlx::Error);

impl From<sqlx::Error> for ReadinessError {
    fn from(error: sqlx::Error) -> Self {
        Self(error)
    }
}

impl IntoResponse for ReadinessError {
    fn into_response(self) -> Response {
        (StatusCode::SERVICE_UNAVAILABLE, self.to_string()).into_response()
    }
}

async fn shutdown_signal() {
    signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C signal handler");
    info!("shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::parse_env;

    #[test]
    fn parses_default_values() {
        let value: u64 = parse_env("THY_SQUEAL_TEST_UNSET_VALUE", "42").expect("valid default");

        assert_eq!(value, 42);
    }
}
