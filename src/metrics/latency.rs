use std::sync::atomic::{AtomicU64, Ordering};

use crate::metrics::LATENCY_BUCKETS_MS;

pub(crate) fn bucket_for(ms: u64) -> usize {
    for (index, bound) in LATENCY_BUCKETS_MS.iter().enumerate() {
        if ms < *bound {
            return index;
        }
    }
    LATENCY_BUCKETS_MS.len()
}

pub(crate) fn mean_ms(requests: u64, sum_ns: u64) -> f64 {
    if requests == 0 {
        return 0.0;
    }
    sum_ns as f64 / requests as f64 / 1_000_000.0
}

pub(crate) fn percentile_ms(buckets: &[AtomicU64], max_ns: u64, percent: u64) -> f64 {
    let total: u64 = buckets
        .iter()
        .map(|bucket| bucket.load(Ordering::Relaxed))
        .sum();
    if total == 0 {
        return 0.0;
    }
    let target = total * percent / 100 + 1;
    let mut cumulative = 0u64;
    for (index, bucket) in buckets.iter().enumerate() {
        cumulative += bucket.load(Ordering::Relaxed);
        if cumulative >= target {
            return if index < LATENCY_BUCKETS_MS.len() {
                LATENCY_BUCKETS_MS[index] as f64
            } else {
                max_ns as f64 / 1_000_000.0
            };
        }
    }
    0.0
}
