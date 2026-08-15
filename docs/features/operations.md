# Operations: Serving SQLite over HTTP

## Purpose

ThySqueal is a lightweight JSON-over-HTTP front for SQLite. This document
describes how to operate it safely: the SQL access policy boundary, resource
limits, and the caveats specific to exposing a SQLite database over HTTP. It
complements [Configuration](configuration.md), [HTTP Query API](http-api.md),
and [SQLite Storage](sqlite-storage.md).

## Security Boundary

- The server has **no built-in authentication, authorization, or TLS**. Any
  client that can reach it can execute policy-allowed SQL against every
  configured database and read whatever that SQL returns. Treat the server as a
  trusted internal component.
- Bind to localhost (`127.0.0.1`) or place the server behind an
  authenticated, TLS-terminating reverse proxy. Never expose the raw HTTP
  listener to the public internet.
- Data returned by queries is served verbatim to any client. Do not put
  secrets in reachable tables if the listener is not strictly protected.
- Error responses are sanitized: they never contain stack traces, database
  file paths, or connection internals. See [HTTP Query API](http-api.md).
- The server and the SQLite files it opens share the operating system user;
  restrict filesystem access so an unprivileged service account owns the
  database directory.

## SQL Access Policy

Raw SQL is parsed and classified before execution; classification is
fail-closed and never trusts keywords or comments.

- **Read-only statements:** `SELECT`, compound selects, and `WITH` queries
  whose body selects data.
- **Data-changing statements:** `INSERT`, `UPDATE`, `DELETE`, and `REPLACE`.
- **Rejected with `422 policy_rejection`:** everything else, including DDL
  (`CREATE`, `DROP`, `ALTER`), transaction control (`BEGIN`, `COMMIT`,
  `ROLLBACK`), `PRAGMA`, `ATTACH`, `DETACH`, `VACUUM`, and `EXPLAIN`.
- Squeal compiles to `SELECT`, so it always classifies as read-only.
- SQLite extension loading is disabled at the connection level.
- SQLite's grammar is richer than the parser's; rare SQLite-only syntax may be
  rejected even though a future SQLite version could accept it.

See [SQLite Storage](sqlite-storage.md) for the full classification rules.

Because DDL is rejected, **schema changes cannot be made through the API**.
Create or migrate tables out-of-band (for example, with the `sqlite3` CLI)
while the server is stopped, and rely on file permissions or the service
account to protect the schema from the API boundary.

## Statement and Execution Caveats

- **Requests are not atomic when they contain multiple statements.** Raw SQL
  may contain several statements and transaction control is rejected, so if a
  later statement fails, earlier statements in the same request remain
  applied. For atomic single-statement writes, send one statement per request;
  SQLite's own statement-level atomicity still holds.
- Requests run concurrently across the pool's connections; SQLite itself
  serializes writes at the database level, so concurrent writers contend.
- SQLite is a single-writer database. The busy timeout is 5 seconds; under
  sustained write contention, requests can fail with `503 unavailable` rather
  than queue indefinitely.
- **There is no query cost limit.** An eligible `SELECT` can scan an entire
  table. Create appropriate indexes and treat `request.timeout_seconds` as the
  hard per-request bound.
- Blob columns are outside the value model; a query returning one fails with
  `unsupported_column`.
- The server is not a substitute for SQLite's own operational tooling: it has
  no backup, replication, or migration features.

## Resource Limits

Configured limits, with defaults, from [Configuration](configuration.md):

- `request.body_limit_bytes` — `1048576` bytes maximum request body.
- `request.timeout_seconds` — `30` seconds per request.
- `long_poll.timeout_seconds` — `30` seconds maximum long-poll wait.
- `long_poll.max_waiters` — `1000` concurrent waiters across all clients.
- `long_poll.max_waiters_per_client` — `10` concurrent waiters per client.
- `cache.max_entries` — `1000` cached results per database; `0` disables
  caching. The cache holds at most `max_entries` entries; collection thresholds
  and intervals bound memory further.

Right-size `long_poll.max_waiters` and per-client limits against the process
file-descriptor limit, because each waiter holds an open connection.

## Caching Caveats

- Cached reads are invalidated after every successful write, so the cache is
  correctness-first: it never serves stale data across a write.
- Non-deterministic reads (`random()`, `current_timestamp`, `datetime()`, and
  friends) bypass the cache entirely.
- Without a configured collection interval, the cache grows toward
  `max_entries` and then stops storing new results; memory stays bounded by
  the entry limit.

## Long Polling Caveats

- Events are published only after a successful write commits, to a bounded
  per-database channel.
- **Events are not replayed.** A waiter that subscribes after an event was
  published will not receive that past event; if it lags a fast writer it may
  see nothing until the next matching write.
- Waiters hold their connection for the full wait; the waiter limits are the
  backstop against exhausted connections.

## Database File Operations

- Databases are opened at startup and their connection settings are fixed for
  the process lifetime; configuration or schema changes require a restart.
- The journal mode is WAL. **Do not delete or replace the database file while
  the server is running.** Back up with SQLite's backup API or a WAL-aware
  snapshot, not a plain file copy.
- Avoid placing SQLite files on network filesystems; SQLite relies on
  filesystem locking that network storage often cannot guarantee.
- Startup fails clearly if a configured database cannot be opened; verify
  filesystem permissions after changing the service account.

## Monitoring

- `GET /healthz` reports process liveness; `GET /readyz` verifies every
  configured database answers a probe query.
- `GET /api/diagnostics` exposes request counts and latency, SQLite failures,
  cache counters and size, and active long-poll waiters; `GET /diagnostics`
  renders the same data. See [Diagnostics](diagnostics.md).
- Structured logs cover request completion, database errors, cache
  collection, and long-poll lifecycle; watch the `error` and `warn` levels
  for execution failures and pool timeouts.

## Acceptance Criteria

- An operator can reason about the access boundary and the resource limits
  that bound memory, connections, and request cost.
- An operator knows that writes should be single statements when atomicity
  matters, and that schema and backups are managed out-of-band.
- An operator can use the health, readiness, diagnostics, and logs to identify
  failures, cache effectiveness, and leaked waiters.
