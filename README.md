# ThySqueal

ThySqueal is a lightweight JSON-over-HTTP server for executing parameterized
SQL against SQLite. It includes an in-memory cache for recent `SELECT` queries
and support for long-polling clients.

## Documentation

- [Product requirements](docs/PRD.md) — project goals, API outline, storage,
  caching, long polling, and testing scope.
- [Implementation TODO](docs/TODO.md) — milestone-based development plan.
- [HTTP SQL API](docs/features/http-api.md)
- [SQLite storage](docs/features/sqlite-storage.md)
- [In-memory select cache](docs/features/select-cache.md)
- [Long polling](docs/features/long-polling.md)
- [Python end-to-end testing](docs/features/end-to-end-testing.md)

## License

This project is licensed under the [MIT License](LICENSE).
