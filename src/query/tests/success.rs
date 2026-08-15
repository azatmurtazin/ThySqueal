use std::collections::HashMap;

use serde_json::json;

use super::{memory_pool, post_json, seed_items, test_router};

#[tokio::test]
async fn runs_parameterized_select() {
    let pool = memory_pool().await;
    seed_items(&pool).await;
    let app = test_router(HashMap::from([("main".to_owned(), pool.clone())]));

    let (status, body) = post_json(
        &app,
        json!({
            "sql": "SELECT id, name, price FROM items WHERE price > ? ORDER BY id",
            "params": [5.0]
        }),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["meta"]["columns"], json!(["id", "name", "price"]));
    assert_eq!(body["meta"]["row_count"], 1);
    assert_eq!(
        body["rows"][0],
        json!({ "id": 1, "name": "widget", "price": 9.99 })
    );
    assert!(body["meta"]["rows_affected"].is_null());
}

#[tokio::test]
async fn reports_write_metadata() {
    let pool = memory_pool().await;
    seed_items(&pool).await;
    let app = test_router(HashMap::from([("main".to_owned(), pool.clone())]));

    let (status, body) = post_json(
        &app,
        json!({
            "sql": "INSERT INTO items (name, price) VALUES (?, ?)",
            "params": ["gizmo", 2.0]
        }),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["meta"]["rows_affected"], 1);
    assert_eq!(body["meta"]["last_insert_id"], 3);
    assert!(body["meta"]["columns"].is_null());
    assert_eq!(body["rows"], json!([]));
}

#[tokio::test]
async fn runs_squeal_select() {
    let pool = memory_pool().await;
    seed_items(&pool).await;
    let app = test_router(HashMap::from([("main".to_owned(), pool.clone())]));

    let (status, body) = post_json(
        &app,
        json!({ "squeal": { "_": "select", "from": "items", "cols": ["id", "name"] } }),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["meta"]["columns"], json!(["id", "name"]));
    assert_eq!(body["meta"]["row_count"], 2);
    assert_eq!(body["rows"][0], json!({ "id": 1, "name": "widget" }));
}

#[tokio::test]
async fn selects_configured_database() {
    let main = memory_pool().await;
    seed_items(&main).await;
    let catalog = memory_pool().await;
    sqlx::query("CREATE TABLE books (id INTEGER PRIMARY KEY, title TEXT)")
        .execute(&catalog)
        .await
        .expect("create books");
    sqlx::query("INSERT INTO books (title) VALUES ('The Rust Book')")
        .execute(&catalog)
        .await
        .expect("insert book");
    let app = test_router(HashMap::from([
        ("main".to_owned(), main),
        ("catalog".to_owned(), catalog),
    ]));

    let (status, body) = post_json(
        &app,
        json!({
            "db": "catalog",
            "sql": "SELECT title FROM books WHERE id = ?",
            "params": [1]
        }),
    )
    .await;

    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["rows"][0]["title"], json!("The Rust Book"));
}
