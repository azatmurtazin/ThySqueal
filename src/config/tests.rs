use super::Config;

#[test]
fn parses_full_configuration() {
    let config = Config::from_str(
        r#"
        bind_address: "127.0.0.1:8080"
        database:
          path: "/tmp/test.db"
          max_connections: 3
        request:
          body_limit_bytes: 1024
          timeout_seconds: 5
        long_poll:
          timeout_seconds: 10
        cache:
          max_entries: 42
        "#,
    )
    .expect("valid configuration");

    assert_eq!(config.bind_address.to_string(), "127.0.0.1:8080");
    assert_eq!(config.database_path().to_str(), Some("/tmp/test.db"));
    assert_eq!(config.database_max_connections(), 3);
    assert_eq!(config.request_body_limit_bytes(), 1024);
    assert_eq!(config.request_timeout().as_secs(), 5);
    assert_eq!(config.long_poll_timeout().as_secs(), 10);
    assert_eq!(config.cache_max_entries(), 42);
}

#[test]
fn uses_defaults_for_missing_fields() {
    let config = Config::from_str(
        r#"
        bind_address: "127.0.0.1:8080"
        cache:
          max_entries: 42
        "#,
    )
    .expect("valid configuration");

    assert_eq!(config.database_path().to_str(), Some("db/thy-squeal.db"));
    assert_eq!(config.database_max_connections(), 5);
    assert_eq!(config.request_body_limit_bytes(), 1048576);
    assert_eq!(config.request_timeout().as_secs(), 30);
    assert_eq!(config.long_poll_timeout().as_secs(), 30);
    assert_eq!(config.cache_max_entries(), 42);
}

#[test]
fn empty_configuration_uses_all_defaults() {
    let config = Config::from_str("").expect("valid empty configuration");

    assert_eq!(config.bind_address.to_string(), "127.0.0.1:5931");
    assert_eq!(config.database_max_connections(), 5);
    assert_eq!(config.request_body_limit_bytes(), 1048576);
    assert_eq!(config.request_timeout().as_secs(), 30);
    assert_eq!(config.long_poll_timeout().as_secs(), 30);
    assert_eq!(config.cache_max_entries(), 1000);
}

#[test]
fn rejects_invalid_configuration() {
    let result = Config::from_str("database:\n  max_connections: not-a-number");

    assert!(result.is_err());
}
