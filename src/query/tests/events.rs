use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Router,
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use serde_json::{Value as JsonValue, json};
use tokio::sync::watch;
use tower::ServiceExt;

use crate::app::AppState;
use crate::cache::{CacheSettings, SelectCache};
use crate::config::Config;
use crate::database::Database;
use crate::events::WaiterLimits;
use crate::query::tests::{memory_pool, post_json, seed_items, test_database};

fn events_router(
    databases: HashMap<String, Database>,
    long_poll_timeout: Duration,
    max_waiters: u64,
    max_waiters_per_client: u64,
) -> (Router, watch::Sender<bool>) {
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let state = AppState {
        databases,
        waiters: Arc::new(WaiterLimits::new(max_waiters, max_waiters_per_client)),
        shutdown: shutdown_rx,
        long_poll_timeout,
    };
    (crate::app::router(state, &Config::default()), shutdown_tx)
}

async fn main_router(long_poll_timeout: Duration) -> (Router, watch::Sender<bool>) {
    let pool = memory_pool().await;
    seed_items(&pool).await;
    let cache = Arc::new(SelectCache::new(CacheSettings::default()));
    events_router(
        HashMap::from([("main".to_owned(), test_database(pool, cache))]),
        long_poll_timeout,
        1000,
        10,
    )
}

async fn limited_router(
    long_poll_timeout: Duration,
    max_waiters: u64,
    max_waiters_per_client: u64,
) -> (Router, watch::Sender<bool>) {
    let pool = memory_pool().await;
    seed_items(&pool).await;
    let cache = Arc::new(SelectCache::new(CacheSettings::default()));
    events_router(
        HashMap::from([("main".to_owned(), test_database(pool, cache))]),
        long_poll_timeout,
        max_waiters,
        max_waiters_per_client,
    )
}

async fn get_events_from(app: &Router, addr: SocketAddr, query: &str) -> (StatusCode, JsonValue) {
    let mut request = Request::builder()
        .method("GET")
        .uri(format!("/api/events{query}"))
        .body(Body::empty())
        .expect("request body");
    request.extensions_mut().insert(ConnectInfo(addr));
    let response = app.clone().oneshot(request).await.expect("request handled");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let body = serde_json::from_slice(bytes.as_ref()).unwrap_or(JsonValue::Null);
    (status, body)
}

async fn get_events(app: &Router, query: &str) -> (StatusCode, JsonValue) {
    get_events_from(app, SocketAddr::from(([127, 0, 0, 1], 9000)), query).await
}

#[tokio::test]
async fn returns_matching_event_after_write() {
    let (app, _shutdown) = main_router(Duration::from_secs(5)).await;
    let app_for_waiter = app.clone();
    let waiter =
        tokio::spawn(async move { get_events(&app_for_waiter, "?db=main&table=items").await });

    tokio::time::sleep(Duration::from_millis(50)).await;
    let (status, _) = post_json(
        &app,
        json!({ "sql": "INSERT INTO items (name, price) VALUES ('gizmo', 2.0)" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = waiter.await.expect("waiter completed");
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["meta"]["database"], "main");
    assert_eq!(body["events"].as_array().expect("events").len(), 1);
    let event = &body["events"][0];
    assert_eq!(event["database"], "main");
    assert_eq!(event["table"], "items");
    assert!(event["at"].as_u64().is_some());
}

#[tokio::test]
async fn times_out_when_no_event_arrives() {
    let (app, _shutdown) = main_router(Duration::from_millis(100)).await;
    let (status, body) = get_events(&app, "?db=main").await;
    assert_eq!(status, StatusCode::REQUEST_TIMEOUT);
    assert_eq!(body["error"]["code"], "long_poll_timeout");
}

#[tokio::test]
async fn table_filter_ignores_non_matching_events() {
    let (app, _shutdown) = main_router(Duration::from_millis(100)).await;
    let app_for_waiter = app.clone();
    let waiter =
        tokio::spawn(async move { get_events(&app_for_waiter, "?db=main&table=other").await });

    tokio::time::sleep(Duration::from_millis(20)).await;
    let (status, _) = post_json(
        &app,
        json!({ "sql": "INSERT INTO items (name, price) VALUES ('gizmo', 2.0)" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = waiter.await.expect("waiter completed");
    assert_eq!(status, StatusCode::REQUEST_TIMEOUT);
    assert_eq!(body["error"]["code"], "long_poll_timeout");
}

#[tokio::test]
async fn unknown_database_is_rejected() {
    let (app, _shutdown) = main_router(Duration::from_secs(1)).await;
    let (status, body) = get_events(&app, "?db=missing").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "unknown_database");
}

#[tokio::test]
async fn invalid_filters_are_rejected() {
    let (app, _shutdown) = main_router(Duration::from_secs(1)).await;

    for query in ["?db=main&limit=0", "?db=main&limit=101", "?db=main&table="] {
        let (status, body) = get_events(&app, query).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "query {query}");
        assert_eq!(body["error"]["code"], "invalid_request", "query {query}");
    }
}

#[tokio::test]
async fn total_waiter_limit_is_enforced() {
    let (app, _shutdown) = limited_router(Duration::from_millis(400), 1, 10).await;
    let first_addr = SocketAddr::from(([127, 0, 0, 1], 9001));
    let second_addr = SocketAddr::from(([127, 0, 0, 1], 9002));

    let app_for_waiter = app.clone();
    let first =
        tokio::spawn(async move { get_events_from(&app_for_waiter, first_addr, "?db=main").await });
    tokio::time::sleep(Duration::from_millis(20)).await;

    let (status, body) = get_events_from(&app, second_addr, "?db=main").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"]["code"], "too_many_waiters");

    first.abort();
}

#[tokio::test]
async fn per_client_waiter_limit_is_enforced() {
    let (app, _shutdown) = limited_router(Duration::from_millis(300), 1000, 1).await;
    let addr = SocketAddr::from(([127, 0, 0, 1], 9003));
    let app_for_waiter = app.clone();
    let first =
        tokio::spawn(async move { get_events_from(&app_for_waiter, addr, "?db=main").await });
    tokio::time::sleep(Duration::from_millis(20)).await;

    let (status, body) = get_events_from(&app, addr, "?db=main").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["error"]["code"], "too_many_waiters");

    first.abort();
}

#[tokio::test]
async fn shutdown_releases_waiters_with_error() {
    let (app, shutdown_tx) = main_router(Duration::from_secs(5)).await;
    let app_for_waiter = app.clone();
    let waiter = tokio::spawn(async move { get_events(&app_for_waiter, "?db=main").await });

    tokio::time::sleep(Duration::from_millis(20)).await;
    let _ = shutdown_tx.send(true);

    let (status, body) = waiter.await.expect("waiter completed");
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"]["code"], "shutting_down");
}
