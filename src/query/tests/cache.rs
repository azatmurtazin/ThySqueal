use std::collections::HashMap;
use std::sync::Arc;

use axum::http::StatusCode;
use serde_json::json;

use crate::cache::SelectCache;
use crate::query::tests::{memory_pool, post_json, seed_items, test_router_with_cache};

async fn seeded_cache(max_entries: u64) -> (axum::Router, Arc<SelectCache>) {
    let pool = memory_pool().await;
    seed_items(&pool).await;
    let cache = Arc::new(SelectCache::new(max_entries));
    let app = test_router_with_cache(
        HashMap::from([("main".to_owned(), pool)]),
        Arc::clone(&cache),
    );
    (app, cache)
}

#[tokio::test]
async fn second_identical_select_hits_cache() {
    let (app, cache) = seeded_cache(1000).await;

    let (status, body) =
        post_json(&app, json!({ "sql": "SELECT name FROM items ORDER BY id" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["meta"]["row_count"], 2);

    let (status, _) = post_json(&app, json!({ "sql": "SELECT name FROM items ORDER BY id" })).await;
    assert_eq!(status, StatusCode::OK);

    let counters = cache.counters();
    assert_eq!(counters.stores, 1);
    assert_eq!(counters.hits, 1);
    assert_eq!(counters.misses, 1);
}

#[tokio::test]
async fn successful_write_invalidates_cached_selects() {
    let (app, cache) = seeded_cache(1000).await;

    let (status, _) = post_json(&app, json!({ "sql": "SELECT count(*) AS n FROM items" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cache.counters().stores, 1);

    let (status, _) = post_json(
        &app,
        json!({ "sql": "INSERT INTO items (name, price) VALUES ('extra', 1.0)" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = post_json(&app, json!({ "sql": "SELECT count(*) AS n FROM items" })).await;
    assert_eq!(status, StatusCode::OK);

    let counters = cache.counters();
    assert_eq!(counters.invalidations, 1);
    assert_eq!(counters.stores, 2);
    assert_eq!(counters.hits, 0);
}

#[tokio::test]
async fn does_not_cache_nondeterministic_selects() {
    let (app, cache) = seeded_cache(1000).await;

    for _ in 0..2 {
        let (status, _) = post_json(&app, json!({ "sql": "SELECT random() AS r" })).await;
        assert_eq!(status, StatusCode::OK);
    }

    let counters = cache.counters();
    assert_eq!(counters.stores, 0);
    assert_eq!(counters.hits, 0);
    assert_eq!(counters.misses, 0);
}

#[tokio::test]
async fn raw_sql_and_squeal_share_cache_entries() {
    let (app, cache) = seeded_cache(1000).await;

    let (status, _) = post_json(&app, json!({ "sql": "SELECT name FROM items" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cache.counters().stores, 1);

    let (status, _) = post_json(
        &app,
        json!({ "squeal": { "_": "select", "from": "items", "cols": ["name"] } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let counters = cache.counters();
    assert_eq!(counters.hits, 1);
    assert_eq!(counters.stores, 1);
}

#[tokio::test]
async fn keys_distinguish_parameter_types() {
    let (app, cache) = seeded_cache(1000).await;

    for params in [json!([1]), json!(["1"]), json!([1.0])] {
        let (status, _) =
            post_json(&app, json!({ "sql": "SELECT ? AS x", "params": params })).await;
        assert_eq!(status, StatusCode::OK);
    }

    let counters = cache.counters();
    assert_eq!(counters.stores, 3);
    assert_eq!(counters.hits, 0);
}
