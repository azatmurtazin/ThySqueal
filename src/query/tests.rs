mod errors;
mod success;

use std::collections::HashMap;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value as JsonValue;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use tower::ServiceExt;

use crate::app::AppState;
use crate::config::Config;

pub(crate) async fn memory_pool() -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory pool")
}

pub(crate) fn test_router(databases: HashMap<String, SqlitePool>) -> Router {
    crate::app::router(AppState { databases }, &Config::default())
}

pub(crate) async fn seed_items(pool: &SqlitePool) {
    sqlx::query("CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT NOT NULL, price REAL)")
        .execute(pool)
        .await
        .expect("create items");
    sqlx::query("INSERT INTO items (name, price) VALUES ('widget', 9.99)")
        .execute(pool)
        .await
        .expect("insert widget");
    sqlx::query("INSERT INTO items (name, price) VALUES ('gadget', 3.5)")
        .execute(pool)
        .await
        .expect("insert gadget");
}

pub(crate) async fn post_json(app: &Router, request: JsonValue) -> (StatusCode, JsonValue) {
    post(app, Some("application/json"), request.to_string()).await
}

pub(crate) async fn post(
    app: &Router,
    content_type: Option<&str>,
    body: String,
) -> (StatusCode, JsonValue) {
    let mut builder = Request::builder().method("POST").uri("/api/query");
    if let Some(content_type) = content_type {
        builder = builder.header("content-type", content_type);
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body)).expect("request body"))
        .await
        .expect("request handled");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let body = serde_json::from_slice(bytes.as_ref()).unwrap_or(JsonValue::Null);
    (status, body)
}
