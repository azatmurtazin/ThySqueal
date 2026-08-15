use std::{
    env, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use axum::{
    Router,
    extract::State,
    http::{HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
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

const DEFAULT_CONFIG_PATH: &str = "thy-squeal.yaml";
const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:3000";
const DEFAULT_DATABASE_PATH: &str = "db/thy-squeal.db";
const DEFAULT_MAX_CONNECTIONS: u32 = 5;
const DEFAULT_REQUEST_BODY_LIMIT_BYTES: usize = 1048576;
const DEFAULT_REQUEST_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_LONG_POLL_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_CACHE_MAX_ENTRIES: u64 = 1000;

#[derive(Clone)]
struct AppState {
    database: SqlitePool,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct Config {
    bind_address: SocketAddr,
    database: DatabaseConfig,
    request: RequestConfig,
    long_poll: LongPollConfig,
    cache: CacheConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_address: DEFAULT_BIND_ADDRESS
                .parse()
                .expect("default bind address is valid"),
            database: DatabaseConfig::default(),
            request: RequestConfig::default(),
            long_poll: LongPollConfig::default(),
            cache: CacheConfig::default(),
        }
    }
}

impl Config {
    fn load(path: &Path) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_owned(),
            source,
        })?;
        Self::from_str(&contents).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source,
        })
    }

    fn from_str(contents: &str) -> Result<Self, serde_yml::Error> {
        if contents.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_yml::from_str(contents)
    }

    fn database_path(&self) -> &Path {
        &self.database.path
    }

    fn database_max_connections(&self) -> u32 {
        self.database.max_connections
    }

    fn request_body_limit_bytes(&self) -> usize {
        self.request.body_limit_bytes
    }

    fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request.timeout_seconds)
    }

    fn long_poll_timeout(&self) -> Duration {
        Duration::from_secs(self.long_poll.timeout_seconds)
    }

    fn cache_max_entries(&self) -> u64 {
        self.cache.max_entries
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct DatabaseConfig {
    path: PathBuf,
    max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from(DEFAULT_DATABASE_PATH),
            max_connections: DEFAULT_MAX_CONNECTIONS,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct RequestConfig {
    body_limit_bytes: usize,
    timeout_seconds: u64,
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            body_limit_bytes: DEFAULT_REQUEST_BODY_LIMIT_BYTES,
            timeout_seconds: DEFAULT_REQUEST_TIMEOUT_SECONDS,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct LongPollConfig {
    timeout_seconds: u64,
}

impl Default for LongPollConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: DEFAULT_LONG_POLL_TIMEOUT_SECONDS,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
struct CacheConfig {
    max_entries: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_CACHE_MAX_ENTRIES,
        }
    }
}

fn config_path_from_args() -> Result<PathBuf, ConfigError> {
    let mut args = env::args().skip(1);
    let mut config_path = PathBuf::from(DEFAULT_CONFIG_PATH);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                config_path = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or(ConfigError::MissingConfigArgument)?;
            }
            other => return Err(ConfigError::UnknownArgument(other.to_owned())),
        }
    }

    Ok(config_path)
}

#[derive(Debug, Error)]
enum ConfigError {
    #[error("could not read configuration file {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("could not parse configuration file {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_yml::Error,
    },
    #[error("--config requires a path argument")]
    MissingConfigArgument,
    #[error("unknown command line argument: {0}")]
    UnknownArgument(String),
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
    let config_path = config_path_from_args()?;
    let config = Config::load(&config_path)?;
    let database = open_database(&config).await?;
    let state = AppState { database };
    let application = app(state, &config);
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
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("axum server is infallible");

    Ok(())
}

async fn open_database(config: &Config) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(config.database_path())
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    SqlitePoolOptions::new()
        .max_connections(config.database_max_connections())
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
                .layer(RequestBodyLimitLayer::new(
                    config.request_body_limit_bytes(),
                ))
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    config.request_timeout(),
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
    use super::Config;

    #[test]
    fn parses_full_configuration() {
        let config = Config::from_str(
            r#"
            bind_address: "127.0.0.1:8080"
            database:
              path: "/tmp/test.db"
              max_connections: 3
            request:
              body_limit_bytes: 1024
              timeout_seconds: 5
            long_poll:
              timeout_seconds: 10
            cache:
              max_entries: 42
            "#,
        )
        .expect("valid configuration");

        assert_eq!(config.bind_address.to_string(), "127.0.0.1:8080");
        assert_eq!(config.database_path().to_str(), Some("/tmp/test.db"));
        assert_eq!(config.database_max_connections(), 3);
        assert_eq!(config.request_body_limit_bytes(), 1024);
        assert_eq!(config.request_timeout().as_secs(), 5);
        assert_eq!(config.long_poll_timeout().as_secs(), 10);
        assert_eq!(config.cache_max_entries(), 42);
    }

    #[test]
    fn uses_defaults_for_missing_fields() {
        let config = Config::from_str(
            r#"
            bind_address: "127.0.0.1:8080"
            cache:
              max_entries: 42
            "#,
        )
        .expect("valid configuration");

        assert_eq!(config.database_path().to_str(), Some("db/thy-squeal.db"));
        assert_eq!(config.database_max_connections(), 5);
        assert_eq!(config.request_body_limit_bytes(), 1048576);
        assert_eq!(config.request_timeout().as_secs(), 30);
        assert_eq!(config.long_poll_timeout().as_secs(), 30);
        assert_eq!(config.cache_max_entries(), 42);
    }

    #[test]
    fn empty_configuration_uses_all_defaults() {
        let config = Config::from_str("").expect("valid empty configuration");

        assert_eq!(config.bind_address.to_string(), "127.0.0.1:3000");
        assert_eq!(config.database_max_connections(), 5);
        assert_eq!(config.request_body_limit_bytes(), 1048576);
        assert_eq!(config.request_timeout().as_secs(), 30);
        assert_eq!(config.long_poll_timeout().as_secs(), 30);
        assert_eq!(config.cache_max_entries(), 1000);
    }

    #[test]
    fn rejects_invalid_configuration() {
        let result = Config::from_str("database:\n  max_connections: not-a-number");

        assert!(result.is_err());
    }
}
