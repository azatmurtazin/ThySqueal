use axum::http::StatusCode;

pub(crate) async fn health_handler() -> StatusCode {
    StatusCode::NO_CONTENT
}
