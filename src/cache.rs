#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use dashmap::DashMap;
use serde_json::Value as JsonValue;

use crate::value::Value;

pub(crate) type CacheKey = Vec<u8>;

pub(crate) struct CachedResult {
    pub(crate) columns: Vec<String>,
    pub(crate) rows: Vec<JsonValue>,
}

impl CachedResult {
    fn size_bytes(&self) -> usize {
        let columns = self.columns.iter().map(String::len).sum::<usize>();
        let rows = self
            .rows
            .iter()
            .map(|value| value.to_string().len())
            .sum::<usize>();
        columns + rows + 64
    }
}

struct Entry {
    result: Arc<CachedResult>,
    size_bytes: usize,
    created: Instant,
    last_access: Instant,
    marked: bool,
}

#[derive(Default)]
struct Counters {
    hits: AtomicU64,
    misses: AtomicU64,
    stores: AtomicU64,
    invalidations: AtomicU64,
    collection_runs: AtomicU64,
    swept_entries: AtomicU64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CounterSnapshot {
    pub(crate) hits: u64,
    pub(crate) misses: u64,
    pub(crate) stores: u64,
    pub(crate) invalidations: u64,
    pub(crate) collection_runs: u64,
    pub(crate) swept_entries: u64,
}

pub(crate) struct SelectCache {
    entries: DashMap<CacheKey, Entry>,
    max_entries: u64,
    counters: Counters,
}

impl SelectCache {
    pub(crate) fn new(max_entries: u64) -> Self {
        Self {
            entries: DashMap::new(),
            max_entries,
            counters: Counters::default(),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn lookup(&self, key: &CacheKey) -> Option<Arc<CachedResult>> {
        let mut entry = self.entries.get_mut(key)?;
        entry.last_access = Instant::now();
        entry.marked = true;
        self.counters.hits.fetch_add(1, Ordering::Relaxed);
        Some(Arc::clone(&entry.result))
    }

    pub(crate) fn record_miss(&self) {
        self.counters.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn store(&self, key: CacheKey, result: CachedResult) -> Option<Arc<CachedResult>> {
        if self.max_entries == 0 || self.entries.len() as u64 >= self.max_entries {
            return None;
        }
        let size_bytes = result.size_bytes();
        let now = Instant::now();
        let result = Arc::new(result);
        self.entries.insert(
            key,
            Entry {
                result: Arc::clone(&result),
                size_bytes,
                created: now,
                last_access: now,
                marked: false,
            },
        );
        self.counters.stores.fetch_add(1, Ordering::Relaxed);
        Some(result)
    }

    pub(crate) fn invalidate_all(&self) {
        self.entries.clear();
        self.counters.invalidations.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn counters(&self) -> CounterSnapshot {
        CounterSnapshot {
            hits: self.counters.hits.load(Ordering::Relaxed),
            misses: self.counters.misses.load(Ordering::Relaxed),
            stores: self.counters.stores.load(Ordering::Relaxed),
            invalidations: self.counters.invalidations.load(Ordering::Relaxed),
            collection_runs: self.counters.collection_runs.load(Ordering::Relaxed),
            swept_entries: self.counters.swept_entries.load(Ordering::Relaxed),
        }
    }
}

pub(crate) fn build_key(sql: &str, params: &[Value]) -> CacheKey {
    let mut key = Vec::with_capacity(sql.len() + params.len() * 16);
    push_str(&mut key, sql);
    for param in params {
        match param {
            Value::Null => key.push(0),
            Value::Boolean(boolean) => {
                key.push(1);
                key.push(u8::from(*boolean));
            }
            Value::Integer(integer) => {
                key.push(2);
                key.extend_from_slice(&integer.to_le_bytes());
            }
            Value::Float(float) => {
                key.push(3);
                key.extend_from_slice(&float.to_bits().to_le_bytes());
            }
            Value::Text(text) => {
                key.push(4);
                key.extend_from_slice(&(text.len() as u64).to_le_bytes());
                key.extend_from_slice(text.as_bytes());
            }
        }
    }
    key
}

fn push_str(key: &mut Vec<u8>, text: &str) {
    key.extend_from_slice(&(text.len() as u64).to_le_bytes());
    key.extend_from_slice(text.as_bytes());
}

#[cfg(test)]
mod tests;
