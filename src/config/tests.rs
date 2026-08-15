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
fn parses_per_database_cache_configuration() {
    let config = Config::from_str(
        r#"
        databases:
          - name: main
            path: "/tmp/main.db"
            cache:
              max_entries: 0
              max_age_seconds: 5
          - name: catalog
            path: "/tmp/catalog.db"
        cache:
          max_entries: 42
          max_age_seconds: 60
          collection_threshold_entries: 20
          collection_threshold_bytes: 4096
          collection_interval_seconds: 30
        "#,
    )
    .expect("valid configuration");

    let global = config.cache_settings();
    assert_eq!(global.max_entries, 42);
    assert_eq!(global.max_age_seconds, 60);
    assert_eq!(global.collection_threshold_entries, 20);
    assert_eq!(global.collection_threshold_bytes, 4096);
    assert_eq!(global.collection_interval_seconds, 30);

    let databases = config.databases();
    let main = databases[0].cache_settings(&global);
    assert_eq!(main.max_entries, 0);
    assert_eq!(main.max_age_seconds, 5);
    assert_eq!(main.collection_threshold_entries, 20);
    let catalog = databases[1].cache_settings(&global);
    assert_eq!(catalog.max_entries, 42);
    assert_eq!(catalog.max_age_seconds, 60);
}

#[test]
fn cache_settings_use_document_defaults() {
    let config = Config::from_str("").expect("valid empty configuration");
    let settings = config.cache_settings();
    assert_eq!(settings.max_entries, 1000);
    assert_eq!(settings.max_age_seconds, 0);
    assert_eq!(settings.collection_threshold_entries, 0);
    assert_eq!(settings.collection_threshold_bytes, 0);
    assert_eq!(settings.collection_interval_seconds, 0);
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
