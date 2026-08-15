use std::time::Duration;

use super::{CacheSettings, CachedResult, SelectCache, build_key};
use crate::value::Value;

fn result() -> CachedResult {
    CachedResult {
        columns: vec!["n".to_owned()],
        rows: vec![serde_json::json!({ "n": 1 })],
    }
}

fn cache(max_entries: u64) -> SelectCache {
    SelectCache::new(CacheSettings::with_max_entries(max_entries))
}

#[test]
fn keys_distinguish_parameter_types() {
    let integer = build_key("SELECT ?", &[Value::Integer(1)]);
    let float = build_key("SELECT ?", &[Value::Float(1.0)]);
    let text = build_key("SELECT ?", &[Value::Text("1".to_owned())]);
    let boolean = build_key("SELECT ?", &[Value::Boolean(true)]);
    let null = build_key("SELECT ?", &[Value::Null]);

    let keys = [integer, float, text, boolean, null];
    for (index, left) in keys.iter().enumerate() {
        for right in &keys[index + 1..] {
            assert_ne!(left, right);
        }
    }
}

#[test]
fn keys_include_sql() {
    assert_ne!(build_key("SELECT 1", &[]), build_key("SELECT 2", &[]));
}

#[test]
fn keys_are_deterministic_for_equal_inputs() {
    assert_eq!(
        build_key("SELECT ?", &[Value::Text("x".to_owned())]),
        build_key("SELECT ?", &[Value::Text("x".to_owned())])
    );
}

#[test]
fn store_lookup_and_invalidate_round_trip() {
    let cache = cache(100);
    let key = build_key("SELECT 1", &[]);
    assert!(cache.lookup(&key).is_none());

    cache.store(key.clone(), result());
    let cached = cache.lookup(&key).expect("cached entry");
    assert_eq!(cached.columns, vec!["n".to_owned()]);
    assert_eq!(cache.counters().stores, 1);
    assert_eq!(cache.counters().hits, 1);

    cache.invalidate_all();
    assert!(cache.lookup(&key).is_none());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.counters().invalidations, 1);
}

#[test]
fn store_skips_entries_when_at_capacity() {
    let cache = cache(1);
    cache.store(build_key("SELECT 1", &[]), result());
    cache.store(build_key("SELECT 2", &[]), result());
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.counters().stores, 1);
}

#[test]
fn zero_capacity_stores_nothing() {
    let cache = cache(0);
    cache.store(build_key("SELECT 1", &[]), result());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.counters().stores, 0);
}

#[test]
fn collection_sweeps_unmarked_entries() {
    let cache = cache(100);
    let first = build_key("SELECT 1", &[]);
    let second = build_key("SELECT 2", &[]);
    cache.store(first.clone(), result());
    cache.store(second.clone(), result());

    cache.collect();
    assert_eq!(cache.len(), 2);
    assert_eq!(cache.counters().swept_entries, 0);

    cache.lookup(&first);
    cache.collect();
    assert_eq!(cache.len(), 1);
    assert!(cache.lookup(&first).is_some());
    assert!(cache.lookup(&second).is_none());
    assert_eq!(cache.counters().collection_runs, 2);
    assert_eq!(cache.counters().swept_entries, 1);
}

#[test]
fn collection_removes_expired_entries() {
    let settings = CacheSettings {
        max_age: Duration::from_millis(5),
        ..Default::default()
    };
    let cache = SelectCache::new(settings);
    cache.store(build_key("SELECT 1", &[]), result());

    std::thread::sleep(Duration::from_millis(15));
    cache.collect();
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.counters().swept_entries, 1);
}

#[test]
fn store_triggers_collection_at_entry_threshold() {
    let settings = CacheSettings {
        max_entries: 100,
        collection_threshold: 2,
        ..Default::default()
    };
    let cache = SelectCache::new(settings);

    cache.store(build_key("SELECT 1", &[]), result());
    cache.store(build_key("SELECT 2", &[]), result());
    assert_eq!(cache.counters().collection_runs, 0);

    cache.store(build_key("SELECT 3", &[]), result());
    assert_eq!(cache.counters().collection_runs, 1);
}

#[test]
fn store_triggers_collection_at_byte_threshold() {
    let settings = CacheSettings {
        max_entries: 100,
        collection_threshold_bytes: 10,
        ..Default::default()
    };
    let cache = SelectCache::new(settings);

    cache.store(build_key("SELECT 1", &[]), result());
    assert_eq!(cache.counters().collection_runs, 0);

    cache.store(build_key("SELECT 2", &[]), result());
    assert_eq!(cache.counters().collection_runs, 1);
}
