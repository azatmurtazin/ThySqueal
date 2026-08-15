use axum::extract::State;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::app::AppState;

pub(crate) async fn ready_handler(
    State(state): State<AppState>,
) -> Result<StatusCode, ReadinessError> {
    for database in state.databases.values() {
        sqlx::query("SELECT 1").execute(&database.pool).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Error)]
#[error("database is unavailable")]
pub(crate) struct ReadinessError(#[source] sqlx::Error);

impl From<sqlx::Error> for ReadinessError {
    fn from(error: sqlx::Error) -> Self {
        Self(error)
    }
}

impl IntoResponse for ReadinessError {
    fn into_response(self) -> Response {
        (StatusCode::SERVICE_UNAVAILABLE, self.to_string()).into_response()
    }
}
