use std::sync::Arc;

use axum::Router;
use axum::http::{HeaderValue, Request, StatusCode};
use tower_http::{
    catch_panic::CatchPanicLayer,
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use uuid::Uuid;

use crate::app::AppState;
use crate::config::Config;
use crate::metrics;

pub(crate) fn apply_middlewares(router: Router, state: &AppState, config: &Config) -> Router {
    let metrics = Arc::clone(&state.metrics);

    router
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
        ))
        .layer(metrics::MetricsLayer::new(metrics))
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
