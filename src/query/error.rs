use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::execution;

#[derive(Debug)]
pub(crate) enum QueryError {
    InvalidRequest(String),
    UnknownDatabase(String),
    SquealUnsupported,
    Execution(execution::Error),
}

impl QueryError {
    pub(crate) fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest(message.into())
    }
}

impl IntoResponse for QueryError {
    fn into_response(self) -> Response {
        let (status, detail) = match self {
            Self::InvalidRequest(message) => (
                StatusCode::BAD_REQUEST,
                ErrorDetail::new("invalid_request", message),
            ),
            Self::UnknownDatabase(name) => (
                StatusCode::BAD_REQUEST,
                ErrorDetail::new("unknown_database", format!("unknown database '{name}'")),
            ),
            Self::SquealUnsupported => (
                StatusCode::BAD_REQUEST,
                ErrorDetail::new("squeal_unsupported", "squeal is not yet supported"),
            ),
            Self::Execution(execution::Error::InvalidQuery(message)) => (
                StatusCode::BAD_REQUEST,
                ErrorDetail::new("invalid_sql", message),
            ),
            Self::Execution(execution::Error::Constraint(message)) => (
                StatusCode::BAD_REQUEST,
                ErrorDetail::new("constraint_violation", message),
            ),
            Self::Execution(execution::Error::Unavailable(message)) => (
                StatusCode::SERVICE_UNAVAILABLE,
                ErrorDetail::new("unavailable", message),
            ),
            Self::Execution(execution::Error::UnsupportedColumn(name)) => (
                StatusCode::BAD_REQUEST,
                ErrorDetail::new(
                    "unsupported_column",
                    format!("unsupported column type: {name}"),
                ),
            ),
            Self::Execution(execution::Error::Execution(message)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorDetail::new("execution_failed", message),
            ),
        };
        (status, Json(ErrorBody { error: detail })).into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl ErrorDetail {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
