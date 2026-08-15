use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

use super::{Error, execute};
use crate::value::Value;

async fn pool() -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory pool")
}

#[tokio::test]
async fn creates_table_inserts_and_queries() {
    let pool = pool().await;

    let created = execute(
        &pool,
        "CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL, price REAL, active INTEGER, note TEXT)",
        &[],
    )
    .await
    .expect("create table");
    assert!(created.rows.is_empty());
    assert!(created.columns.is_empty());

    let inserted = execute(
        &pool,
        "INSERT INTO items (name, price, active, note) VALUES (?, ?, ?, ?)",
        &[
            Value::Text("widget".to_owned()),
            Value::Float(9.99),
            Value::Boolean(true),
            Value::Null,
        ],
    )
    .await
    .expect("insert");
    assert_eq!(inserted.rows_affected, 1);
    assert_eq!(inserted.last_insert_id, 1);

    let selected = execute(
        &pool,
        "SELECT id, name, price, active, note FROM items WHERE id = ?",
        &[Value::Integer(1)],
    )
    .await
    .expect("select");

    assert_eq!(
        selected.columns,
        vec!["id", "name", "price", "active", "note"]
    );
    assert_eq!(selected.rows.len(), 1);
    let row = &selected.rows[0];
    assert_eq!(row[0], Value::Integer(1));
    assert_eq!(row[1], Value::Text("widget".to_owned()));
    assert_eq!(row[2], Value::Float(9.99));
    assert_eq!(row[3], Value::Integer(1));
    assert_eq!(row[4], Value::Null);
}

#[tokio::test]
async fn rejects_invalid_sql() {
    let pool = pool().await;

    let result = execute(&pool, "SELECT * FROM missing_table", &[]).await;

    assert!(matches!(result, Err(Error::InvalidQuery(_))));
}

#[tokio::test]
async fn surfaces_constraint_failures() {
    let pool = pool().await;
    execute(
        &pool,
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        &[],
    )
    .await
    .expect("create table");

    let result = execute(
        &pool,
        "INSERT INTO t (id, name) VALUES (?, ?)",
        &[Value::Integer(1), Value::Null],
    )
    .await;

    assert!(matches!(result, Err(Error::Constraint(_))));
}

#[tokio::test]
async fn binds_different_value_types() {
    let pool = pool().await;
    execute(
        &pool,
        "CREATE TABLE t (i INTEGER, f REAL, s TEXT, n TEXT)",
        &[],
    )
    .await
    .expect("create table");

    execute(
        &pool,
        "INSERT INTO t VALUES (?, ?, ?, ?)",
        &[
            Value::Integer(42),
            Value::Float(3.5),
            Value::Text("hello".to_owned()),
            Value::Null,
        ],
    )
    .await
    .expect("insert");

    let selected = execute(&pool, "SELECT i, f, s, n FROM t", &[])
        .await
        .expect("select");
    let row = &selected.rows[0];
    assert_eq!(row[0], Value::Integer(42));
    assert_eq!(row[1], Value::Float(3.5));
    assert_eq!(row[2], Value::Text("hello".to_owned()));
    assert_eq!(row[3], Value::Null);
}
