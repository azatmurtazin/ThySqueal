#![allow(dead_code)]

use futures_util::TryStreamExt;
use sqlx::{
    Column, Decode,
    Either::{Left, Right},
    Row, Sqlite, SqlitePool, TypeInfo, ValueRef,
    sqlite::{SqliteRow, SqliteValueRef},
};
use thiserror::Error;

use crate::value::Value;

pub(crate) struct ExecutionResult {
    pub(crate) columns: Vec<String>,
    pub(crate) rows: Vec<Vec<Value>>,
    pub(crate) rows_affected: u64,
    pub(crate) last_insert_id: i64,
}

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("invalid query: {0}")]
    InvalidQuery(String),
    #[error("constraint violation: {0}")]
    Constraint(String),
    #[error("database is unavailable: {0}")]
    Unavailable(String),
    #[error("query execution failed: {0}")]
    Execution(String),
    #[error("unsupported column type: {0}")]
    UnsupportedColumn(String),
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        match &error {
            sqlx::Error::Database(database) => {
                let message = database.message().to_owned();
                if is_invalid_query(&message) {
                    Self::InvalidQuery(message)
                } else if is_constraint(&message) {
                    Self::Constraint(message)
                } else if message.contains("database is locked") {
                    Self::Unavailable(message)
                } else {
                    Self::Execution(message)
                }
            }
            sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed => {
                Self::Unavailable(error.to_string())
            }
            _ => Self::Execution(error.to_string()),
        }
    }
}

fn is_invalid_query(message: &str) -> bool {
    message.starts_with("near \"")
        || message.contains("syntax error")
        || message.contains("no such table")
        || message.contains("no such column")
}

fn is_constraint(message: &str) -> bool {
    message.contains("constraint failed")
        || message.contains("UNIQUE constraint")
        || message.contains("NOT NULL constraint")
        || message.contains("PRIMARY KEY constraint")
        || message.contains("FOREIGN KEY constraint")
        || message.contains("CHECK constraint")
}

pub(crate) async fn execute(
    pool: &SqlitePool,
    sql: &str,
    params: &[Value],
) -> Result<ExecutionResult, Error> {
    let query = params
        .iter()
        .fold(sqlx::query(sqlx::AssertSqlSafe(sql)), |query, value| {
            query.bind(value.clone())
        });

    #[allow(deprecated)]
    let mut stream = query.fetch_many(pool);
    let mut rows: Vec<SqliteRow> = Vec::new();
    let mut rows_affected: u64 = 0;
    let mut last_insert_id: i64 = 0;

    while let Some(item) = stream.try_next().await? {
        match item {
            Left(result) => {
                rows_affected = result.rows_affected();
                last_insert_id = result.last_insert_rowid();
            }
            Right(row) => rows.push(row),
        }
    }

    let columns = row_columns(&rows);
    let rows = decode_rows(&rows)?;

    Ok(ExecutionResult {
        columns,
        rows,
        rows_affected,
        last_insert_id,
    })
}

fn row_columns(rows: &[SqliteRow]) -> Vec<String> {
    rows.first()
        .map(|row| {
            row.columns()
                .iter()
                .map(|column| column.name().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn decode_rows(rows: &[SqliteRow]) -> Result<Vec<Vec<Value>>, Error> {
    rows.iter()
        .map(|row| {
            (0..row.len())
                .map(|index| {
                    let raw = row
                        .try_get_raw(index)
                        .map_err(|error| Error::Execution(error.to_string()))?;
                    value_from_sqlite(raw)
                })
                .collect()
        })
        .collect()
}

fn value_from_sqlite(value: SqliteValueRef<'_>) -> Result<Value, Error> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    match value.type_info().name() {
        "INTEGER" => {
            let integer = <i64 as Decode<Sqlite>>::decode(value)
                .map_err(|error| Error::Execution(error.to_string()))?;
            Ok(Value::Integer(integer))
        }
        "REAL" => {
            let float = <f64 as Decode<Sqlite>>::decode(value)
                .map_err(|error| Error::Execution(error.to_string()))?;
            Ok(Value::Float(float))
        }
        "TEXT" => {
            let text = <String as Decode<Sqlite>>::decode(value)
                .map_err(|error| Error::Execution(error.to_string()))?;
            Ok(Value::Text(text))
        }
        name => Err(Error::UnsupportedColumn(name.to_owned())),
    }
}

#[cfg(test)]
mod tests;
