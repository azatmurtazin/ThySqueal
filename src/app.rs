use axum::{
    Router,
    extract::State,
    http::{HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use thiserror::Error;
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use uuid::Uuid;

use crate::config::Config;
use crate::database::Registry;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) databases: Registry,
}

pub(crate) fn router(state: AppState, config: &Config) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(readiness))
        .route("/api/query", post(crate::query::execute_query))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(CatchPanicLayer::new())
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(PropagateRequestIdLayer::x_request_id())
                .layer(RequestBodyLimitLayer::new(
                    config.request_body_limit_bytes(),
                ))
                .layer(TraceLayer::new_for_http())
                .layer(TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    config.request_timeout(),
                )),
        )
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn readiness(State(state): State<AppState>) -> Result<StatusCode, ReadinessError> {
    for pool in state.databases.values() {
        sqlx::query("SELECT 1").execute(pool).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Error)]
#[error("database is unavailable")]
struct ReadinessError(#[source] sqlx::Error);

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

#[derive(Clone)]
struct MakeRequestUuid;

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        HeaderValue::from_str(&Uuid::new_v4().to_string())
            .ok()
            .map(RequestId::new)
    }
}
