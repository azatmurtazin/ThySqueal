use std::{
    env, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::Deserialize;
use thiserror::Error;

pub(crate) const DEFAULT_CONFIG_PATH: &str = "thy-squeal.yaml";
const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:5931";
const DEFAULT_DATABASE_PATH: &str = "db/thy-squeal.db";
const DEFAULT_MAX_CONNECTIONS: u32 = 5;
const DEFAULT_REQUEST_BODY_LIMIT_BYTES: usize = 1048576;
const DEFAULT_REQUEST_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_LONG_POLL_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_CACHE_MAX_ENTRIES: u64 = 1000;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub(crate) struct Config {
    pub(crate) bind_address: SocketAddr,
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
    pub(crate) fn load(path: &Path) -> Result<Self, ConfigError> {
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

    pub(crate) fn database_path(&self) -> &Path {
        &self.database.path
    }

    pub(crate) fn database_max_connections(&self) -> u32 {
        self.database.max_connections
    }

    pub(crate) fn request_body_limit_bytes(&self) -> usize {
        self.request.body_limit_bytes
    }

    pub(crate) fn request_timeout(&self) -> Duration {
        Duration::from_secs(self.request.timeout_seconds)
    }

    pub(crate) fn long_poll_timeout(&self) -> Duration {
        Duration::from_secs(self.long_poll.timeout_seconds)
    }

    pub(crate) fn cache_max_entries(&self) -> u64 {
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

pub(crate) fn path_from_args() -> Result<PathBuf, ConfigError> {
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
pub(crate) enum ConfigError {
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

#[cfg(test)]
mod tests;
