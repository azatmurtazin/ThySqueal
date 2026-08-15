use std::{collections::HashMap, sync::Arc, time::Duration};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use thiserror::Error;

use crate::cache::{CacheSettings, SelectCache};
use crate::config::{Config, DatabaseConfig, ResolvedCacheConfig};

#[cfg(test)]
mod tests;

pub(crate) type Registry = HashMap<String, Database>;

#[derive(Clone)]
pub(crate) struct Database {
    pub(crate) pool: SqlitePool,
    pub(crate) cache: Arc<SelectCache>,
}

#[derive(Debug, Error)]
pub(crate) enum OpenError {
    #[error("could not open database {name}: {source}")]
    Database { name: String, source: sqlx::Error },
}

pub(crate) async fn open_all(config: &Config) -> Result<Registry, OpenError> {
    let global_cache = config.cache_settings();
    let mut databases = HashMap::new();
    for database in config.databases() {
        let pool = open(database).await.map_err(|source| OpenError::Database {
            name: database.name.clone(),
            source,
        })?;
        let cache = Arc::new(SelectCache::new(cache_settings(
            &database.cache_settings(&global_cache),
        )));
        cache.clone().spawn_periodic_collection();
        databases.insert(database.name.clone(), Database { pool, cache });
    }
    Ok(databases)
}

fn cache_settings(config: &ResolvedCacheConfig) -> CacheSettings {
    CacheSettings {
        max_entries: config.max_entries,
        max_age: Duration::from_secs(config.max_age_seconds),
        collection_threshold: if config.collection_threshold_entries == 0 {
            config.max_entries
        } else {
            config.collection_threshold_entries
        },
        collection_threshold_bytes: config.collection_threshold_bytes,
        collection_interval: Duration::from_secs(config.collection_interval_seconds),
    }
}

async fn open(database: &DatabaseConfig) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(&database.path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));

    SqlitePoolOptions::new()
        .max_connections(database.max_connections)
        .connect_with(options)
        .await
}
