#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::Serialize;
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub(crate) struct CounterSnapshot {
    pub(crate) hits: u64,
    pub(crate) misses: u64,
    pub(crate) stores: u64,
    pub(crate) invalidations: u64,
    pub(crate) collection_runs: u64,
    pub(crate) swept_entries: u64,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct CacheSettings {
    pub(crate) max_entries: u64,
    pub(crate) max_age: Duration,
    pub(crate) collection_threshold: u64,
    pub(crate) collection_threshold_bytes: u64,
    pub(crate) collection_interval: Duration,
}

impl CacheSettings {
    pub(crate) fn with_max_entries(max_entries: u64) -> Self {
        Self {
            max_entries,
            collection_threshold: max_entries,
            ..Self::default()
        }
    }
}

impl Default for CacheSettings {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            max_age: Duration::ZERO,
            collection_threshold: 1000,
            collection_threshold_bytes: 0,
            collection_interval: Duration::ZERO,
        }
    }
}

pub(crate) struct SelectCache {
    entries: DashMap<CacheKey, Entry>,
    settings: CacheSettings,
    bytes: AtomicU64,
    counters: Counters,
}

impl SelectCache {
    pub(crate) fn new(settings: CacheSettings) -> Self {
        Self {
            entries: DashMap::new(),
            settings,
            bytes: AtomicU64::new(0),
            counters: Counters::default(),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn bytes(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }

    pub(crate) fn max_entries(&self) -> u64 {
        self.settings.max_entries
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
        if self.settings.max_entries == 0 {
            return None;
        }
        if self.threshold_reached() {
            self.collect();
        }
        if self.entries.len() as u64 >= self.settings.max_entries {
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
                marked: true,
            },
        );
        self.bytes.fetch_add(size_bytes as u64, Ordering::Relaxed);
        self.counters.stores.fetch_add(1, Ordering::Relaxed);
        Some(result)
    }

    pub(crate) fn invalidate_all(&self) {
        self.entries.clear();
        self.bytes.store(0, Ordering::Relaxed);
        self.counters.invalidations.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn collect(&self) {
        let started = Instant::now();
        let before = self.entries.len();
        let now = Instant::now();
        let max_age = self.settings.max_age;
        let mut swept: u64 = 0;
        let mut bytes_reclaimed: usize = 0;

        self.entries.retain(|_, entry| {
            let expired =
                !max_age.is_zero() && now.saturating_duration_since(entry.created) >= max_age;
            if !entry.marked || expired {
                swept += 1;
                bytes_reclaimed += entry.size_bytes;
                return false;
            }
            entry.marked = false;
            true
        });

        if swept > 0 {
            self.subtract_bytes(bytes_reclaimed as u64);
        }
        self.counters
            .collection_runs
            .fetch_add(1, Ordering::Relaxed);
        self.counters
            .swept_entries
            .fetch_add(swept, Ordering::Relaxed);
        tracing::debug!(
            duration_ms = started.elapsed().as_millis(),
            before,
            after = before.saturating_sub(swept as usize),
            bytes_reclaimed,
            swept,
            "select cache collection",
        );
    }

    pub(crate) fn spawn_periodic_collection(self: Arc<Self>) {
        let interval = self.settings.collection_interval;
        if interval.is_zero() {
            return;
        }
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                self.collect();
            }
        });
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

    fn threshold_reached(&self) -> bool {
        self.entries.len() as u64 >= self.settings.collection_threshold
            || (self.settings.collection_threshold_bytes > 0
                && self.bytes.load(Ordering::Relaxed) >= self.settings.collection_threshold_bytes)
    }

    fn subtract_bytes(&self, amount: u64) {
        let _ = self
            .bytes
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(current.saturating_sub(amount))
            });
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
