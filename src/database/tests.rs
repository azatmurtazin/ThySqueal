use std::fs;

use uuid::Uuid;

use crate::config::DatabaseConfig;

#[tokio::test]
async fn applies_deliberate_connection_options() {
    let path = std::env::temp_dir().join(format!("thy-squeal-{}.db", Uuid::new_v4()));
    let database = DatabaseConfig {
        name: "main".to_owned(),
        path: path.clone(),
        max_connections: 1,
        cache: None,
    };
    let pool = super::open(&database).await.expect("open database");

    let mut connection = pool.acquire().await.expect("acquire connection");

    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(&mut *connection)
        .await
        .expect("foreign_keys pragma");
    assert_eq!(foreign_keys, 1);

    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&mut *connection)
        .await
        .expect("journal_mode pragma");
    assert_eq!(journal_mode, "wal");

    let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(&mut *connection)
        .await
        .expect("busy_timeout pragma");
    assert_eq!(busy_timeout, 5000);

    drop(connection);
    pool.close().await;
    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(format!("{}-wal", path.display()));
    let _ = fs::remove_file(format!("{}-shm", path.display()));
}
