use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Snapshot {
    pub(crate) started_at_millis: u64,
    pub(crate) uptime_seconds: f64,
    pub(crate) requests: RequestSnapshot,
    pub(crate) sqlite: SqliteSnapshot,
    pub(crate) long_poll: LongPollSnapshot,
}

#[derive(Debug, Serialize)]
pub(crate) struct RequestSnapshot {
    pub(crate) total: u64,
    pub(crate) in_flight: u64,
    pub(crate) responses_2xx: u64,
    pub(crate) responses_3xx: u64,
    pub(crate) responses_4xx: u64,
    pub(crate) responses_5xx: u64,
    pub(crate) latency: LatencySnapshot,
}

#[derive(Debug, Serialize)]
pub(crate) struct LatencySnapshot {
    pub(crate) mean_ms: f64,
    pub(crate) p50_ms: f64,
    pub(crate) p90_ms: f64,
    pub(crate) p99_ms: f64,
    pub(crate) max_ms: f64,
}

#[derive(Debug, Serialize)]
pub(crate) struct SqliteSnapshot {
    pub(crate) executions: u64,
    pub(crate) errors: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct LongPollSnapshot {
    pub(crate) waits: u64,
    pub(crate) timeouts: u64,
    pub(crate) shutdowns: u64,
    pub(crate) rejected_total: u64,
    pub(crate) rejected_per_client: u64,
    pub(crate) events_published: u64,
}
