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
bind_address: "127.0.0.1:3000"

database:
  path: "db/thy-squeal.db"
  max_connections: 5

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
| `bind_address` | `127.0.0.1:3000` | Socket address the HTTP server binds to. |
| `database.path` | `db/thy-squeal.db` | SQLite database file location; created if missing. |
| `database.max_connections` | `5` | Maximum connections in the SQLite pool. |
| `request.body_limit_bytes` | `1048576` | Maximum accepted request body size. |
| `request.timeout_seconds` | `30` | Per-request timeout for the HTTP layer. |
| `long_poll.timeout_seconds` | `30` | Maximum wait duration for a long-poll request. |
| `cache.max_entries` | `1000` | Upper bound for cached select-query entries. |

## Validation

- The file must contain valid YAML that matches the schema above.
- Values must parse as their documented types (for example, `max_connections`
  must be a number).
- An unreadable or unparseable configuration file is a startup error.
- Unknown command line arguments are rejected; `--config` requires a path
  argument.

## Acceptance Criteria

- The server starts with an empty or absent `thy-squeal.yaml` using documented
  defaults.
- Configuration from `thy-squeal.yaml` is applied at startup.
- A missing or invalid configuration file fails startup with a clear error.
