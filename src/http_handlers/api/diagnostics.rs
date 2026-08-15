use axum::{Json, extract::State};
use serde::Serialize;

use crate::app::AppState;
use crate::cache::CounterSnapshot;
use crate::metrics;

pub(crate) async fn diagnostics_handler(State(state): State<AppState>) -> Json<Report> {
    Json(report(&state))
}

fn report(state: &AppState) -> Report {
    let snapshot = state.metrics.snapshot();
    let databases = state
        .databases
        .iter()
        .map(|(name, database)| DatabaseReport {
            name: name.clone(),
            pool_connections: database.pool.size(),
            pool_idle: database.pool.num_idle() as u64,
            cache_entries: database.cache.len(),
            cache_bytes: database.cache.bytes(),
            cache_max_entries: database.cache.max_entries(),
            counters: database.cache.counters(),
        })
        .collect();
    Report {
        started_at_millis: snapshot.started_at_millis,
        uptime_seconds: snapshot.uptime_seconds,
        requests: snapshot.requests,
        sqlite: snapshot.sqlite,
        long_poll: LongPollReport {
            active: state.waiters.active(),
            max: state.waiters.max(),
            max_per_client: state.waiters.max_per_client(),
            waits: snapshot.long_poll.waits,
            timeouts: snapshot.long_poll.timeouts,
            shutdowns: snapshot.long_poll.shutdowns,
            rejected_total: snapshot.long_poll.rejected_total,
            rejected_per_client: snapshot.long_poll.rejected_per_client,
            events_published: snapshot.long_poll.events_published,
        },
        databases,
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Report {
    started_at_millis: u64,
    uptime_seconds: f64,
    requests: metrics::RequestSnapshot,
    sqlite: metrics::SqliteSnapshot,
    long_poll: LongPollReport,
    databases: Vec<DatabaseReport>,
}

#[derive(Debug, Serialize)]
struct LongPollReport {
    active: u64,
    max: u64,
    max_per_client: u64,
    waits: u64,
    timeouts: u64,
    shutdowns: u64,
    rejected_total: u64,
    rejected_per_client: u64,
    events_published: u64,
}

#[derive(Debug, Serialize)]
struct DatabaseReport {
    name: String,
    pool_connections: u32,
    pool_idle: u64,
    cache_entries: usize,
    cache_bytes: u64,
    cache_max_entries: u64,
    counters: CounterSnapshot,
}
