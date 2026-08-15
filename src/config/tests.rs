use super::Config;

#[test]
fn parses_full_configuration() {
    let config = Config::from_str(
        r#"
        bind_address: "127.0.0.1:8080"
        databases:
          - name: main
            path: "/tmp/main.db"
            max_connections: 3
          - name: catalog
            path: "/tmp/catalog.db"
            max_connections: 7
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
    let databases = config.databases();
    assert_eq!(databases.len(), 2);
    assert_eq!(databases[0].name, "main");
    assert_eq!(databases[0].path.to_str(), Some("/tmp/main.db"));
    assert_eq!(databases[0].max_connections, 3);
    assert_eq!(databases[1].name, "catalog");
    assert_eq!(databases[1].path.to_str(), Some("/tmp/catalog.db"));
    assert_eq!(databases[1].max_connections, 7);
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
        databases:
          - name: catalog
            path: "/tmp/catalog.db"
        cache:
          max_entries: 42
        "#,
    )
    .expect("valid configuration");

    assert_eq!(config.bind_address.to_string(), "127.0.0.1:8080");
    let databases = config.databases();
    assert_eq!(databases.len(), 1);
    assert_eq!(databases[0].name, "catalog");
    assert_eq!(databases[0].path.to_str(), Some("/tmp/catalog.db"));
    assert_eq!(databases[0].max_connections, 5);
    assert_eq!(config.request_body_limit_bytes(), 1048576);
    assert_eq!(config.request_timeout().as_secs(), 30);
    assert_eq!(config.long_poll_timeout().as_secs(), 30);
    assert_eq!(config.cache_max_entries(), 42);
}

#[test]
fn missing_databases_uses_default_database() {
    let config = Config::from_str("").expect("valid empty configuration");

    assert_eq!(config.bind_address.to_string(), "127.0.0.1:5931");
    let databases = config.databases();
    assert_eq!(databases.len(), 1);
    assert_eq!(databases[0].name, "main");
    assert_eq!(databases[0].path.to_str(), Some("db/thy-squeal.db"));
    assert_eq!(databases[0].max_connections, 5);
    assert_eq!(config.request_body_limit_bytes(), 1048576);
    assert_eq!(config.request_timeout().as_secs(), 30);
    assert_eq!(config.long_poll_timeout().as_secs(), 30);
    assert_eq!(config.cache_max_entries(), 1000);
}

#[test]
fn rejects_invalid_configuration() {
    let result = Config::from_str("databases:\n  - name: main\n    max_connections: not-a-number");

    assert!(result.is_err());
}

#[test]
fn rejects_duplicate_database_names() {
    let config = Config::from_str(
        r#"
        databases:
          - name: main
            path: "/tmp/main.db"
          - name: main
            path: "/tmp/other.db"
        "#,
    )
    .expect("valid yaml");

    assert!(config.validate().is_err());
}
