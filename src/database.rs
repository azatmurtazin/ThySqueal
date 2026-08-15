use std::{collections::HashMap, sync::Arc, time::Duration};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use thiserror::Error;

use crate::cache::SelectCache;
use crate::config::{Config, DatabaseConfig};

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
    let mut databases = HashMap::new();
    for database in config.databases() {
        let pool = open(database).await.map_err(|source| OpenError::Database {
            name: database.name.clone(),
            source,
        })?;
        databases.insert(
            database.name.clone(),
            Database {
                pool,
                cache: Arc::new(SelectCache::new(
                    database.cache_max_entries(config.cache_max_entries()),
                )),
            },
        );
    }
    Ok(databases)
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
