use std::path::PathBuf;

use serde::Deserialize;

const DEFAULT_DATABASE_PATH: &str = "db/thy-squeal.db";
const DEFAULT_MAX_CONNECTIONS: u32 = 5;
const DEFAULT_REQUEST_BODY_LIMIT_BYTES: usize = 1048576;
const DEFAULT_REQUEST_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_LONG_POLL_TIMEOUT_SECONDS: u64 = 30;
const DEFAULT_LONG_POLL_MAX_WAITERS: u64 = 1000;
const DEFAULT_LONG_POLL_MAX_WAITERS_PER_CLIENT: u64 = 10;
const DEFAULT_CACHE_MAX_ENTRIES: u64 = 1000;

#[derive(Debug, Deserialize)]
#[serde(default)]
pub(crate) struct DatabaseConfig {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) max_connections: u32,
    pub(crate) cache: Option<CacheConfig>,
}

impl DatabaseConfig {
    pub(crate) fn cache_settings(&self, global: &ResolvedCacheConfig) -> ResolvedCacheConfig {
        let Some(per_database) = &self.cache else {
            return *global;
        };
        ResolvedCacheConfig {
            max_entries: per_database.max_entries.unwrap_or(global.max_entries),
            max_age_seconds: per_database
                .max_age_seconds
                .unwrap_or(global.max_age_seconds),
            collection_threshold_entries: per_database
                .collection_threshold_entries
                .unwrap_or(global.collection_threshold_entries),
            collection_threshold_bytes: per_database
                .collection_threshold_bytes
                .unwrap_or(global.collection_threshold_bytes),
            collection_interval_seconds: per_database
                .collection_interval_seconds
                .unwrap_or(global.collection_interval_seconds),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            name: "main".to_owned(),
            path: PathBuf::from(DEFAULT_DATABASE_PATH),
            max_connections: DEFAULT_MAX_CONNECTIONS,
            cache: None,
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
    pub(crate) max_waiters: u64,
    pub(crate) max_waiters_per_client: u64,
}

impl Default for LongPollConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: DEFAULT_LONG_POLL_TIMEOUT_SECONDS,
            max_waiters: DEFAULT_LONG_POLL_MAX_WAITERS,
            max_waiters_per_client: DEFAULT_LONG_POLL_MAX_WAITERS_PER_CLIENT,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResolvedCacheConfig {
    pub(crate) max_entries: u64,
    pub(crate) max_age_seconds: u64,
    pub(crate) collection_threshold_entries: u64,
    pub(crate) collection_threshold_bytes: u64,
    pub(crate) collection_interval_seconds: u64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub(crate) struct CacheConfig {
    pub(crate) max_entries: Option<u64>,
    pub(crate) max_age_seconds: Option<u64>,
    pub(crate) collection_threshold_entries: Option<u64>,
    pub(crate) collection_threshold_bytes: Option<u64>,
    pub(crate) collection_interval_seconds: Option<u64>,
}

impl CacheConfig {
    pub(crate) fn resolve(&self) -> ResolvedCacheConfig {
        ResolvedCacheConfig {
            max_entries: self.max_entries.unwrap_or(DEFAULT_CACHE_MAX_ENTRIES),
            max_age_seconds: self.max_age_seconds.unwrap_or(0),
            collection_threshold_entries: self.collection_threshold_entries.unwrap_or(0),
            collection_threshold_bytes: self.collection_threshold_bytes.unwrap_or(0),
            collection_interval_seconds: self.collection_interval_seconds.unwrap_or(0),
        }
    }
}
