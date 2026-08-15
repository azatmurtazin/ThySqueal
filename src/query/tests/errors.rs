use std::collections::HashMap;

use axum::http::StatusCode;
use serde_json::json;

use super::{memory_pool, post, post_json, seed_items, test_router};

#[tokio::test]
async fn rejects_missing_query_field() {
    let app = test_router(HashMap::from([("main".to_owned(), memory_pool().await)]));

    let (status, body) = post_json(&app, json!({ "params": [] })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_request");
}

#[tokio::test]
async fn rejects_both_sql_and_squeal() {
    let app = test_router(HashMap::from([("main".to_owned(), memory_pool().await)]));

    let (status, body) = post_json(
        &app,
        json!({ "sql": "select 1", "squeal": { "_": "select" } }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_request");
}

#[tokio::test]
async fn rejects_empty_sql() {
    let app = test_router(HashMap::from([("main".to_owned(), memory_pool().await)]));

    let (status, body) = post_json(&app, json!({ "sql": "   " })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_request");
}

#[tokio::test]
async fn rejects_params_without_sql() {
    let app = test_router(HashMap::from([("main".to_owned(), memory_pool().await)]));

    let (status, body) = post_json(&app, json!({ "params": [1] })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_request");
}

#[tokio::test]
async fn rejects_params_with_squeal() {
    let app = test_router(HashMap::from([("main".to_owned(), memory_pool().await)]));

    let (status, body) = post_json(
        &app,
        json!({ "squeal": { "_": "select", "from": "items", "cols": ["*"] }, "params": [1] }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_request");
}

#[tokio::test]
async fn rejects_invalid_squeal() {
    let app = test_router(HashMap::from([("main".to_owned(), memory_pool().await)]));

    let (status, body) = post_json(
        &app,
        json!({ "squeal": { "_": "select", "from": "items" } }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_squeal");
}

#[tokio::test]
async fn rejects_unknown_database() {
    let app = test_router(HashMap::from([("main".to_owned(), memory_pool().await)]));

    let (status, body) = post_json(&app, json!({ "db": "missing", "sql": "select 1" })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "unknown_database");
}

#[tokio::test]
async fn rejects_invalid_param_values() {
    let app = test_router(HashMap::from([("main".to_owned(), memory_pool().await)]));

    let (status, body) =
        post_json(&app, json!({ "sql": "select ?", "params": [{ "a": 1 }] })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_request");
}

#[tokio::test]
async fn maps_invalid_sql() {
    let app = test_router(HashMap::from([("main".to_owned(), memory_pool().await)]));

    let (status, body) = post_json(&app, json!({ "sql": "SELECT * FROM missing_table" })).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_sql");
}

#[tokio::test]
async fn maps_constraint_failures() {
    let pool = memory_pool().await;
    seed_items(&pool).await;
    let app = test_router(HashMap::from([("main".to_owned(), pool)]));

    let (status, body) = post_json(
        &app,
        json!({
            "sql": "INSERT INTO items (name, price) VALUES (?, ?)",
            "params": [null, 1.0]
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "constraint_violation");
}

#[tokio::test]
async fn rejects_missing_json_content_type() {
    let app = test_router(HashMap::from([("main".to_owned(), memory_pool().await)]));

    let (status, body) = post(&app, None, "{}".to_owned()).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_request");
}

#[tokio::test]
async fn rejects_malformed_json() {
    let app = test_router(HashMap::from([("main".to_owned(), memory_pool().await)]));

    let (status, body) = post(&app, Some("application/json"), "{not json".to_owned()).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_request");
}
