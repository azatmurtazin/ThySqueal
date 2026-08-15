# Configuration

## Purpose

ThySqueal reads its server configuration from a YAML file instead of
environment variables. The default file is `thy-squeal.yaml` in the current
working directory; a different file can be selected with the `--config <path>`
command line argument.

## Configuration File

> Default port number is `5931 -> 59u3a1 -> squeal`.

Every field is optional. Omitted fields use the documented defaults. An example
with all defaults is checked in as `thy-squeal.yaml`:

```yaml
bind_address: "127.0.0.1:5931"

databases:
  - name: main
    path: "db/thy-squeal.db"
    max_connections: 5
    cache:
      max_entries: 1000

request:
  body_limit_bytes: 1048576
  timeout_seconds: 30

long_poll:
  timeout_seconds: 30
  max_waiters: 1000
  max_waiters_per_client: 10

cache:
  max_entries: 1000
  max_age_seconds: 0
  collection_threshold_entries: 0
  collection_threshold_bytes: 0
  collection_interval_seconds: 0
```

## Values

| Key | Default | Description |
| --- | --- | --- |
| `bind_address` | `127.0.0.1:5931` | Socket address the HTTP server binds to. |
| `databases` | `[main]` | Named SQLite databases exposed by the server. |
| `databases[].name` | `main` | Unique name used to select the database. |
| `databases[].path` | `db/thy-squeal.db` | SQLite database file location; created if missing. |
| `databases[].max_connections` | `5` | Maximum connections in the database's pool. |
| `databases[].cache.max_entries` | global `cache.max_entries` | Upper bound for this database's cached select-query entries. Each database has its own cache. `0` disables caching for that database. |
| `databases[].cache.max_age_seconds` | global `cache.max_age_seconds` | Maximum age of a cached entry for this database. `0` disables expiry. |
| `databases[].cache.collection_threshold_entries` | global `cache.collection_threshold_entries` | Entry count that triggers collection for this database. |
| `databases[].cache.collection_threshold_bytes` | global `cache.collection_threshold_bytes` | Estimated byte count that triggers collection for this database. `0` disables the byte threshold. |
| `databases[].cache.collection_interval_seconds` | global `cache.collection_interval_seconds` | Periodic collection interval for this database. `0` disables the periodic timer. |
| `request.body_limit_bytes` | `1048576` | Maximum accepted request body size. |
| `request.timeout_seconds` | `30` | Per-request timeout for the HTTP layer. |
| `long_poll.timeout_seconds` | `30` | Maximum wait duration for a long-poll request. |
| `long_poll.max_waiters` | `1000` | Maximum number of concurrent long-poll waiters across all clients. |
| `long_poll.max_waiters_per_client` | `10` | Maximum concurrent long-poll waiters for a single client connection. |
| `cache.max_entries` | `1000` | Default upper bound for cached select-query entries, inherited by databases that do not set their own `cache.max_entries`. |
| `cache.max_age_seconds` | `0` | Default maximum age of a cached entry; `0` means entries do not expire. Inherited by databases that do not set their own value. |
| `cache.collection_threshold_entries` | `0` | Default entry count that triggers a mark-and-sweep collection. `0` falls back to the database's `max_entries`. |
| `cache.collection_threshold_bytes` | `0` | Default estimated byte count that triggers collection; `0` disables the byte threshold. |
| `cache.collection_interval_seconds` | `0` | Default periodic collection interval; `0` disables the periodic timer. |

## Validation

- The file must contain valid YAML that matches the schema above.
- Values must parse as their documented types (for example, `max_connections`
  must be a number).
- Database names must be non-empty and unique across the `databases` list.
- When no databases are configured, a single default database named `main` is
  used.
- An unreadable or unparseable configuration file is a startup error.
- Unknown command line arguments are rejected; `--config` requires a path
  argument.

## Acceptance Criteria

- The server starts with an empty or absent `thy-squeal.yaml` using documented
  defaults.
- Configuration from `thy-squeal.yaml` is applied at startup.
- A missing or invalid configuration file fails startup with a clear error.
