use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode},
};
use serde_json::Value as JsonValue;
use tower::ServiceExt;

use crate::cache::{CacheSettings, SelectCache};
use crate::query::tests::{
    memory_pool, post_json, seed_items, test_database, test_router_with_state,
};

async fn diagnostics_router() -> Router {
    let pool = memory_pool().await;
    seed_items(&pool).await;
    let cache = Arc::new(SelectCache::new(CacheSettings::default()));
    test_router_with_state(
        HashMap::from([("main".to_owned(), test_database(pool, cache))]),
        Duration::from_millis(500),
        1000,
        10,
    )
}

async fn get(app: &Router, uri: &str) -> (StatusCode, HeaderMap, String) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, headers, String::from_utf8(bytes.to_vec()).unwrap())
}

#[tokio::test]
async fn diagnostics_json_reports_server_state() {
    let app = diagnostics_router().await;
    let (status, headers, body) = get(&app, "/api/diagnostics").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        headers
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("application/json")
    );

    let report: JsonValue = serde_json::from_str(&body).unwrap();
    assert!(report["started_at_millis"].as_u64().unwrap() > 0);
    assert!(report["uptime_seconds"].as_f64().unwrap() >= 0.0);
    assert_eq!(report["requests"]["in_flight"], JsonValue::from(1));
    assert_eq!(report["long_poll"]["active"], JsonValue::from(0));
    assert_eq!(report["databases"][0]["name"], JsonValue::from("main"));
    assert!(report["databases"][0]["pool_connections"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn diagnostics_reflects_query_activity() {
    let app = diagnostics_router().await;
    post_json(&app, serde_json::json!({ "sql": "SELECT * FROM items" })).await;

    let (status, _, body) = get(&app, "/api/diagnostics").await;
    assert_eq!(status, StatusCode::OK);

    let report: JsonValue = serde_json::from_str(&body).unwrap();
    let requests_total = report["requests"]["total"].as_u64().unwrap();
    let sqlite_executions = report["sqlite"]["executions"].as_u64().unwrap();
    assert!(requests_total >= 1, "total requests {requests_total}");
    assert!(
        sqlite_executions >= 1,
        "sqlite executions {sqlite_executions}"
    );
}

#[tokio::test]
async fn diagnostics_page_serves_html_dashboard() {
    let app = diagnostics_router().await;
    let (status, headers, body) = get(&app, "/diagnostics").await;

    assert_eq!(status, StatusCode::OK);
    assert!(
        headers
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(body.contains("ThySqueal"));
    assert!(body.contains("/api/diagnostics"));
}
