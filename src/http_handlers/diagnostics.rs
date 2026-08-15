use axum::response::{Html, IntoResponse, Response};

pub(crate) async fn diagnostics_handler() -> Response {
    Html(include_str!("../../assets/dashboard.html")).into_response()
}
