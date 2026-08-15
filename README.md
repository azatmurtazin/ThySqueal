# ThySqueal

ThySqueal is a lightweight JSON-over-HTTP server for querying SQLite with raw,
parameterized SQL or Squeal, a structured JSON representation of SQL. It
includes an in-memory cache for recent `SELECT` queries and support for
long-polling clients.

## Technology

The server is written in Rust using Axum and Tokio. It uses Serde for JSON,
SQLx for asynchronous SQLite access, Tower middleware for HTTP limits and
request tracing, and DashMap for the custom mark-and-sweep query cache. See the
[implementation TODO](docs/TODO.md) for the complete dependency plan.

## Documentation

- [Product requirements](docs/PRD.md) — project goals, API outline, storage,
  caching, long polling, and testing scope.
- [Implementation TODO](docs/TODO.md) — milestone-based development plan.
- [HTTP query API](docs/features/http-api.md)
- [Configuration](docs/features/configuration.md)
- [Squeal query language](docs/features/squeal.md)
- [SQLite storage](docs/features/sqlite-storage.md)
- [In-memory select cache](docs/features/select-cache.md)
- [Long polling](docs/features/long-polling.md)
- [Diagnostics and observability](docs/features/diagnostics.md)
- [Operations: SQLite over HTTP](docs/features/operations.md)
- [Python end-to-end testing](docs/features/end-to-end-testing.md)

## License

This project is licensed under the [MIT License](LICENSE).
