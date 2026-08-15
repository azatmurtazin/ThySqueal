use axum::response::Redirect;

pub(crate) async fn home_handler() -> Redirect {
    Redirect::to("/diagnostics")
}
