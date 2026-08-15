use std::path::PathBuf;

use serde::Deserialize;

const DEFAULT_DATABASE_PATH: &str = "db/thy-squeal.db";
const DEFAULT_MAX_CONNECTIONS: u32 = 5;
const DEFAULT_REQUEST_BODY_LIMIT_BYTES: usize = 1048576;
const DEFAULT_REQUEST_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_LONG_POLL_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_CACHE_MAX_ENTRIES: u64 = 1000;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub(crate) struct DatabaseConfig {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            name: "main".to_owned(),
            path: PathBuf::from(DEFAULT_DATABASE_PATH),
            max_connections: DEFAULT_MAX_CONNECTIONS,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub(crate) struct RequestConfig {
    pub(crate) body_limit_bytes: usize,
    pub(crate) timeout_seconds: u64,
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
pub(crate) struct LongPollConfig {
    pub(crate) timeout_seconds: u64,
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
pub(crate) struct CacheConfig {
    pub(crate) max_entries: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_CACHE_MAX_ENTRIES,
        }
    }
}
