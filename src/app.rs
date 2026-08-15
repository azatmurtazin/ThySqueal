use std::sync::Arc;
use std::time::Duration;

use axum::{
    Router,
    routing::{get, post},
};
use tokio::sync::watch;

use crate::database::Registry;
use crate::events::WaiterLimits;
use crate::http_handlers;
use crate::metrics;
use crate::{config::Config, http_handlers::middlewares::apply_middlewares};

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) databases: Registry,
    pub(crate) waiters: Arc<WaiterLimits>,
    pub(crate) shutdown: watch::Receiver<bool>,
    pub(crate) long_poll_timeout: Duration,
    pub(crate) metrics: Arc<metrics::Metrics>,
}

pub(crate) fn router(state: AppState, config: &Config) -> Router {
    let router = Router::new()
        .route("/healthz", get(http_handlers::healthz::health_handler))
        .route("/readyz", get(http_handlers::readyz::ready_handler))
        .route("/api/query", post(crate::query::execute_query))
        .route("/api/privileged-query", post(crate::query::execute_query))
        .route("/api/events", get(crate::events::wait_for_event))
        .route(
            "/api/diagnostics",
            get(http_handlers::api::diagnostics::diagnostics_handler),
        )
        .route(
            "/diagnostics",
            get(http_handlers::diagnostics::diagnostics_handler),
        )
        .route("/", get(http_handlers::home::home_handler))
        .with_state(state.clone());

    apply_middlewares(router, &state, config)
}
