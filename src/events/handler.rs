use std::net::SocketAddr;
use std::time::Duration;

use axum::{
    Json,
    extract::{ConnectInfo, Query, State, rejection::QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::app::AppState;
use crate::events::{ChangeEvent, limits::AcquireError};
use crate::query::QueryError;

const MAX_EVENTS_PER_RESPONSE: u32 = 100;

#[derive(Debug, Deserialize)]
pub(crate) struct EventParams {
    pub(crate) db: Option<String>,
    pub(crate) table: Option<String>,
    pub(crate) limit: Option<u32>,
}

pub(crate) async fn wait_for_event(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    params: Result<Query<EventParams>, QueryRejection>,
    State(state): State<AppState>,
) -> Response {
    let params = match params {
        Ok(Query(params)) => params,
        Err(rejection) => {
            return QueryError::invalid_request(rejection.body_text()).into_response();
        }
    };

    let database_name = params.db.clone().unwrap_or_else(|| "main".to_owned());
    if !state.databases.contains_key(&database_name) {
        return QueryError::UnknownDatabase(database_name).into_response();
    }
    let limit = match params.limit {
        None => 1,
        Some(0) => {
            return QueryError::invalid_request("limit must be between 1 and 100").into_response();
        }
        Some(limit) if limit > MAX_EVENTS_PER_RESPONSE => {
            return QueryError::invalid_request("limit must be between 1 and 100").into_response();
        }
        Some(limit) => limit,
    };
    if params.table.as_deref().is_some_and(str::is_empty) {
        return QueryError::invalid_request("table must not be empty when provided")
            .into_response();
    }

    let _guard = match state.waiters.try_acquire(addr) {
        Ok(guard) => guard,
        Err(AcquireError::Total) => {
            return Error::TooManyWaiters.into_response();
        }
        Err(AcquireError::PerClient) => {
            return Error::PerClientWaitersExceeded.into_response();
        }
    };

    let outcome = wait(&state, database_name.clone(), params.table, limit).await;

    match outcome {
        WaitOutcome::Events(events) => (
            StatusCode::OK,
            Json(EventsResponse {
                meta: EventsMeta {
                    database: database_name,
                },
                events,
            }),
        )
            .into_response(),
        WaitOutcome::Timeout(events) if !events.is_empty() => (
            StatusCode::OK,
            Json(EventsResponse {
                meta: EventsMeta {
                    database: database_name,
                },
                events,
            }),
        )
            .into_response(),
        WaitOutcome::Timeout(_) => Error::Timeout.into_response(),
        WaitOutcome::Shutdown => Error::ShuttingDown.into_response(),
    }
}

enum WaitOutcome {
    Events(Vec<ChangeEvent>),
    Timeout(Vec<ChangeEvent>),
    Shutdown,
}

async fn wait(
    state: &AppState,
    database_name: String,
    table: Option<String>,
    limit: u32,
) -> WaitOutcome {
    let timeout: Duration = state.long_poll_timeout;
    let database = &state.databases[&database_name];
    let mut receiver = database.events.subscribe();
    let mut shutdown = state.shutdown.clone();
    let mut collected = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        tokio::select! {
            biased;
            _ = tokio::time::sleep_until(deadline) => {
                return WaitOutcome::Timeout(collected);
            }
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    return WaitOutcome::Shutdown;
                }
            }
            event = receiver.recv() => {
                match event {
                    Ok(event) => {
                        if table_matches(&table, &event.table) {
                            collected.push(event);
                            if collected.len() as u32 >= limit {
                                return WaitOutcome::Events(collected);
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        return WaitOutcome::Shutdown;
                    }
                }
            }
        }
    }
}
fn table_matches(filter: &Option<String>, event_table: &Option<String>) -> bool {
    match filter {
        None => true,
        Some(filter) => match event_table {
            None => true,
            Some(event_table) => filter == event_table,
        },
    }
}

#[derive(Debug, Serialize)]
struct EventsResponse {
    meta: EventsMeta,
    events: Vec<ChangeEvent>,
}

#[derive(Debug, Serialize)]
struct EventsMeta {
    database: String,
}

#[derive(Debug)]
enum Error {
    Timeout,
    TooManyWaiters,
    PerClientWaitersExceeded,
    ShuttingDown,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Timeout => (
                StatusCode::REQUEST_TIMEOUT,
                "long_poll_timeout",
                "no change event arrived within the configured timeout",
            ),
            Self::TooManyWaiters => (
                StatusCode::SERVICE_UNAVAILABLE,
                "too_many_waiters",
                "the maximum number of concurrent long-poll waiters is reached",
            ),
            Self::PerClientWaitersExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "too_many_waiters",
                "this client already has the maximum number of long-poll waiters",
            ),
            Self::ShuttingDown => (
                StatusCode::SERVICE_UNAVAILABLE,
                "shutting_down",
                "the server is shutting down",
            ),
        };
        (
            status,
            Json(ErrorBody {
                error: ErrorDetail { code, message },
            }),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: &'static str,
}
