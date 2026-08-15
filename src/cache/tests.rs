use super::{CachedResult, SelectCache, build_key};
use crate::value::Value;

fn result() -> CachedResult {
    CachedResult {
        columns: vec!["n".to_owned()],
        rows: vec![serde_json::json!({ "n": 1 })],
    }
}

#[test]
fn keys_distinguish_parameter_types() {
    let integer = build_key("main", "SELECT ?", &[Value::Integer(1)]);
    let float = build_key("main", "SELECT ?", &[Value::Float(1.0)]);
    let text = build_key("main", "SELECT ?", &[Value::Text("1".to_owned())]);
    let boolean = build_key("main", "SELECT ?", &[Value::Boolean(true)]);
    let null = build_key("main", "SELECT ?", &[Value::Null]);

    let keys = [integer, float, text, boolean, null];
    for (index, left) in keys.iter().enumerate() {
        for right in &keys[index + 1..] {
            assert_ne!(left, right);
        }
    }
}

#[test]
fn keys_include_database_and_sql() {
    assert_ne!(
        build_key("main", "SELECT 1", &[]),
        build_key("other", "SELECT 1", &[])
    );
    assert_ne!(
        build_key("main", "SELECT 1", &[]),
        build_key("main", "SELECT 2", &[])
    );
}

#[test]
fn keys_are_deterministic_for_equal_inputs() {
    assert_eq!(
        build_key("main", "SELECT ?", &[Value::Text("x".to_owned())]),
        build_key("main", "SELECT ?", &[Value::Text("x".to_owned())])
    );
}

#[test]
fn store_lookup_and_invalidate_round_trip() {
    let cache = SelectCache::new(100);
    let key = build_key("main", "SELECT 1", &[]);
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
    let cache = SelectCache::new(1);
    cache.store(build_key("main", "SELECT 1", &[]), result());
    cache.store(build_key("main", "SELECT 2", &[]), result());
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.counters().stores, 1);
}

#[test]
fn zero_capacity_stores_nothing() {
    let cache = SelectCache::new(0);
    cache.store(build_key("main", "SELECT 1", &[]), result());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.counters().stores, 0);
}
