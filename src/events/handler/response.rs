use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::events::ChangeEvent;

#[derive(Debug, Serialize)]
pub(super) struct EventsResponse {
    pub(super) meta: EventsMeta,
    pub(super) events: Vec<ChangeEvent>,
}

#[derive(Debug, Serialize)]
pub(super) struct EventsMeta {
    pub(super) database: String,
}

pub(super) enum Error {
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
