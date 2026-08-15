mod latency;
mod layer;
mod snapshot;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::http::StatusCode;

pub(crate) use self::layer::MetricsLayer;
pub(crate) use self::snapshot::{
    LatencySnapshot, LongPollSnapshot, RequestSnapshot, Snapshot, SqliteSnapshot,
};

pub(crate) const LATENCY_BUCKETS_MS: [u64; 7] = [1, 5, 25, 100, 500, 2_000, 10_000];
const NUM_LATENCY_BUCKETS: usize = LATENCY_BUCKETS_MS.len() + 1;

pub(crate) struct Metrics {
    started: Instant,
    started_at: SystemTime,
    requests: AtomicU64,
    in_flight: AtomicU64,
    responses_2xx: AtomicU64,
    responses_3xx: AtomicU64,
    responses_4xx: AtomicU64,
    responses_5xx: AtomicU64,
    latency_ns_sum: AtomicU64,
    latency_ns_max: AtomicU64,
    latency_buckets: [AtomicU64; NUM_LATENCY_BUCKETS],
    sqlite_executions: AtomicU64,
    sqlite_errors: AtomicU64,
    events_published: AtomicU64,
    long_poll_waits: AtomicU64,
    long_poll_timeouts: AtomicU64,
    long_poll_shutdowns: AtomicU64,
    long_poll_rejected_total: AtomicU64,
    long_poll_rejected_per_client: AtomicU64,
}

impl Metrics {
    pub(crate) fn new() -> Self {
        Self {
            started: Instant::now(),
            started_at: SystemTime::now(),
            requests: AtomicU64::new(0),
            in_flight: AtomicU64::new(0),
            responses_2xx: AtomicU64::new(0),
            responses_3xx: AtomicU64::new(0),
            responses_4xx: AtomicU64::new(0),
            responses_5xx: AtomicU64::new(0),
            latency_ns_sum: AtomicU64::new(0),
            latency_ns_max: AtomicU64::new(0),
            latency_buckets: std::array::from_fn(|_| AtomicU64::new(0)),
            sqlite_executions: AtomicU64::new(0),
            sqlite_errors: AtomicU64::new(0),
            events_published: AtomicU64::new(0),
            long_poll_waits: AtomicU64::new(0),
            long_poll_timeouts: AtomicU64::new(0),
            long_poll_shutdowns: AtomicU64::new(0),
            long_poll_rejected_total: AtomicU64::new(0),
            long_poll_rejected_per_client: AtomicU64::new(0),
        }
    }

    pub(crate) fn begin_request(&self) {
        self.in_flight.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_request(&self, status: StatusCode, elapsed: Duration) {
        self.in_flight.fetch_sub(1, Ordering::Relaxed);
        self.requests.fetch_add(1, Ordering::Relaxed);
        if status.is_success() {
            self.responses_2xx.fetch_add(1, Ordering::Relaxed);
        } else if status.is_redirection() {
            self.responses_3xx.fetch_add(1, Ordering::Relaxed);
        } else if status.is_client_error() {
            self.responses_4xx.fetch_add(1, Ordering::Relaxed);
        } else if status.is_server_error() {
            self.responses_5xx.fetch_add(1, Ordering::Relaxed);
        }
        let ns = elapsed.as_nanos() as u64;
        self.latency_ns_sum.fetch_add(ns, Ordering::Relaxed);
        self.latency_ns_max.fetch_max(ns, Ordering::Relaxed);
        self.latency_buckets[latency::bucket_for(elapsed.as_millis() as u64)]
            .fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn cancel_request(&self) {
        self.in_flight.fetch_sub(1, Ordering::Relaxed);
    }

    pub(crate) fn record_sqlite(&self) {
        self.sqlite_executions.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_sqlite_error(&self) {
        self.sqlite_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_event_published(&self) {
        self.events_published.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_long_poll_wait(&self) {
        self.long_poll_waits.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_long_poll_timeout(&self) {
        self.long_poll_timeouts.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_long_poll_shutdown(&self) {
        self.long_poll_shutdowns.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_long_poll_rejected_total(&self) {
        self.long_poll_rejected_total
            .fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_long_poll_rejected_per_client(&self) {
        self.long_poll_rejected_per_client
            .fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        let uptime = self.started.elapsed();
        let max_ns = self.latency_ns_max.load(Ordering::Relaxed);
        Snapshot {
            started_at_millis: self
                .started_at
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            uptime_seconds: uptime.as_secs_f64(),
            requests: RequestSnapshot {
                total: self.requests.load(Ordering::Relaxed),
                in_flight: self.in_flight.load(Ordering::Relaxed),
                responses_2xx: self.responses_2xx.load(Ordering::Relaxed),
                responses_3xx: self.responses_3xx.load(Ordering::Relaxed),
                responses_4xx: self.responses_4xx.load(Ordering::Relaxed),
                responses_5xx: self.responses_5xx.load(Ordering::Relaxed),
                latency: LatencySnapshot {
                    mean_ms: latency::mean_ms(
                        self.requests.load(Ordering::Relaxed),
                        self.latency_ns_sum.load(Ordering::Relaxed),
                    ),
                    p50_ms: latency::percentile_ms(&self.latency_buckets, max_ns, 50),
                    p90_ms: latency::percentile_ms(&self.latency_buckets, max_ns, 90),
                    p99_ms: latency::percentile_ms(&self.latency_buckets, max_ns, 99),
                    max_ms: max_ns as f64 / 1_000_000.0,
                },
            },
            sqlite: SqliteSnapshot {
                executions: self.sqlite_executions.load(Ordering::Relaxed),
                errors: self.sqlite_errors.load(Ordering::Relaxed),
            },
            long_poll: LongPollSnapshot {
                waits: self.long_poll_waits.load(Ordering::Relaxed),
                timeouts: self.long_poll_timeouts.load(Ordering::Relaxed),
                shutdowns: self.long_poll_shutdowns.load(Ordering::Relaxed),
                rejected_total: self.long_poll_rejected_total.load(Ordering::Relaxed),
                rejected_per_client: self.long_poll_rejected_per_client.load(Ordering::Relaxed),
                events_published: self.events_published.load(Ordering::Relaxed),
            },
        }
    }
}
