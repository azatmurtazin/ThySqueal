# Configuration

## Purpose

ThySqueal reads its server configuration from a YAML file instead of
environment variables. The default file is `thy-squeal.yaml` in the current
working directory; a different file can be selected with the `--config <path>`
command line argument.

## Configuration File

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

cache:
  max_entries: 1000
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
| `request.body_limit_bytes` | `1048576` | Maximum accepted request body size. |
| `request.timeout_seconds` | `30` | Per-request timeout for the HTTP layer. |
| `long_poll.timeout_seconds` | `30` | Maximum wait duration for a long-poll request. |
| `cache.max_entries` | `1000` | Default upper bound for cached select-query entries, inherited by databases that do not set their own `cache.max_entries`. |

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
