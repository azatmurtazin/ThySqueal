use tracing::info;

pub(crate) async fn signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C signal handler");
    info!("shutdown signal received");
}
