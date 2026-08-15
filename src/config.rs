mod args;
mod error;
mod sections;

use std::{collections::HashSet, fs, net::SocketAddr, path::Path, time::Duration};

use serde::Deserialize;

use self::sections::{CacheConfig, LongPollConfig, RequestConfig};

pub(crate) use self::args::path_from_args;
pub(crate) use self::error::ConfigError;
pub(crate) use self::sections::DatabaseConfig;

const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:5931";

#[derive(Debug, Deserialize)]
#[serde(default)]
pub(crate) struct Config {
    pub(crate) bind_address: SocketAddr,
    databases: Vec<DatabaseConfig>,
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
            databases: vec![DatabaseConfig::default()],
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
        let config = Self::from_str(&contents).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source,
        })?;
        config.validate().map_err(|message| ConfigError::Invalid {
            path: path.to_owned(),
            message,
        })?;
        Ok(config)
    }

    fn from_str(contents: &str) -> Result<Self, serde_yml::Error> {
        let mut config: Config = if contents.trim().is_empty() {
            Self::default()
        } else {
            serde_yml::from_str(contents)?
        };
        config.ensure_default_database();
        Ok(config)
    }

    fn ensure_default_database(&mut self) {
        if self.databases.is_empty() {
            self.databases.push(DatabaseConfig::default());
        }
    }

    fn validate(&self) -> Result<(), String> {
        let mut names = HashSet::new();
        for database in &self.databases {
            if database.name.trim().is_empty() {
                return Err("database names must not be empty".to_owned());
            }
            if !names.insert(&database.name) {
                return Err(format!("duplicate database name '{}'", database.name));
            }
        }
        Ok(())
    }

    pub(crate) fn databases(&self) -> &[DatabaseConfig] {
        &self.databases
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

#[cfg(test)]
mod tests;
