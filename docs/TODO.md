# Implementation TODO

This checklist turns the product requirements into an incremental implementation
plan. Complete each milestone with its verification steps before proceeding to
the next one.

## 1. Project Foundation

- [x] Add the selected Rust dependencies: Axum, Tokio, Serde, `serde_json`,
  SQLx with SQLite support, Tower, `tower-http`, Tracing,
  `tracing-subscriber`, DashMap, and `thiserror`.
- [x] Configure only the required crate features: Tokio runtime, networking,
  synchronization, timers, and signal handling; SQLx Tokio and SQLite support;
  and Tower HTTP tracing, request-ID, panic-catching, timeout, and size-limit
  layers.
- [x] Define the executable entry point and a configuration model for database
  path, bind address, request limits, cache limits, and long-poll timeout.
- [x] Add structured logging with `tracing`, request IDs from `tower-http`, and
  `tracing-subscriber` environment-filtered log levels.
- [x] Define a server lifecycle: load configuration, open SQLite, initialize
  shared state, listen for requests, and gracefully shut down.
- [x] Add development commands for formatting, linting, unit tests, and running
  the server locally.

**Done when:** the server starts with an empty SQLite database, exposes a
health or readiness endpoint, and exits cleanly on shutdown.

## 2. SQLite Integration

- [x] Open a SQLx `SqlitePool` during startup and surface failures as clear
  startup errors.
- [x] Set deliberate SQLite connection options, busy timeout, and pragmas.
- [x] Create an execution module using SQLx prepared queries that accepts raw
  SQL plus bound values or compiled Squeal, then returns rows, column metadata,
  and write metadata.
- [x] Implement a Squeal parser, validator, and compiler that converts its JSON
  AST into SQLite SQL plus bound values; do not accept arbitrary SQL fragments
  within Squeal fields.
- [x] Model supported JSON values and convert them safely to and from SQLite
  values; booleans map to SQLite integers `0`/`1`, and blobs are not part of
  the public value model.
- [x] Define allowed raw-SQL statement classes and Squeal operations, and
  reject prohibited administrative or extension-loading statements.
- [x] Map SQLite syntax, binding, constraint, busy, and internal errors to
  application error types without leaking database paths or internals.
- [x] Ensure statements, transactions, and connections are released on all
  success, error, timeout, and cancellation paths.

**Done when:** a Rust-level test can create a table, insert bound values,
query rows, and receive correctly typed execution results.

## 3. JSON API: `POST /api/query`

- [x] Set up Axum routes, extractors, application state, and Serde JSON
  request/response types.
- [x] Implement request validation requiring exactly one of a non-empty `sql`
  string or a `squeal` object; accept `params` only with raw `sql`. Squeal is
  compiled and executed; `squeal` with `sql` or `params` is rejected.
- [x] Bind raw `params` through SQLite's parameter API; never construct SQL by
  interpolating client data. Squeal literals introduced by future operations
  are emitted as bound parameters by the compiler.
- [x] Implement the response envelope with `meta` and `rows` for every
  successful statement.
- [x] Return column names and row count for row-producing statements.
- [x] Return affected-row count and last inserted row ID where meaningful for
  non-row statements, with `rows: []`.
- [x] Define stable JSON error objects containing a machine-readable code and a
  safe, client-useful message.
- [x] Map invalid JSON and validation errors to `400`, rejected SQL policy to
  `422`, unavailable database to `503`, and unexpected failures to `500`.
- [x] Set response content type and add Tower request body-size and request
  timeout limits.

**Done when:** a client can execute parameterized raw `SELECT`, `INSERT`, and
`UPDATE`, plus a valid Squeal `select`, and invalid requests through HTTP and
receive documented responses.

## 4. Query Classification and Cache Boundary

- [x] Classify raw SQL and compiled Squeal statements as cacheable read,
  data-changing write, or uncached operation using a conservative policy.
- [x] Initially cache only unambiguous, read-only `SELECT` statements.
- [x] Explicitly bypass caching for non-deterministic or unsupported queries.
- [x] Define a canonical cache key from compiled SQL and a type-preserving
  serialization of all bound parameters, independent of whether the client
  used raw SQL or Squeal.
- [x] Define immutable cached result data matching the HTTP response envelope.
- [x] Store cache entries in DashMap and keep cache operations safe under
  concurrent requests.

**Done when:** logically distinct requests, including requests whose parameters
have different values or types, cannot share a cache entry.

## 5. In-Memory Select Cache

- [x] Implement DashMap cache lookup before SQLx execution for cacheable reads.
- [x] Store successful eligible read results after SQLite execution.
- [x] Track entry size, creation time, last access, and mark state.
- [x] Invalidate all cached selects after each successful write as the initial
  correctness-first policy.
- [x] Add counters for hits, misses, stores, invalidations, collection runs,
  and swept entries.
- [x] Add configuration for the maximum entry count. Collection threshold and
  interval config arrive with the mark-and-sweep collector in Milestone 6.

**Done when:** repeated eligible reads avoid a second database execution and a
successful write prevents any stale cached result from being served.

## 6. Mark-and-Sweep Cache Collection

- [x] Mark cache entries on cache hits and on insertion when appropriate for
  the collection policy.
- [x] Trigger collection when configured memory or entry thresholds are met,
  and optionally on a periodic timer.
- [x] Sweep entries that were not marked in the current collection generation.
- [x] Sweep entries older than a configured maximum age (TTL/max-age), using the
  `created` and `last_access` timestamps already tracked per entry, in addition
  to entries unused since the last collection cycle.
- [x] Clear or advance marks on surviving entries so future collection cycles
  can distinguish recent use from old use.
- [x] Make collection safe when requests read or write cache entries
  concurrently, using DashMap entry operations without holding references
  across async await points.
- [x] Record collection duration, entry count before and after, bytes reclaimed,
  and number of entries swept.

**Done when:** tests show that accessed entries survive one collection cycle,
unused entries are reclaimed, entries older than the configured maximum age are
not served, and configured cache limits remain bounded.

## 7. Long Polling

- [x] Define the long-poll endpoint, request filter, response schema, timeout
  status, and event schema in the public API documentation.
- [x] Implement a registry of pending waiters with cancellation-safe cleanup.
- [x] Use a Tokio `broadcast` channel to publish change events to concurrent
  waiters, applying `tokio::time::timeout` to each wait.
- [x] Publish change events only after successful relevant writes commit.
- [x] Deliver matching events to waiting requests and remove completed waiters.
- [x] Implement a configurable maximum wait duration and normal timeout
  response when no event arrives.
- [x] Detect client disconnects and unregister their waiters promptly.
- [x] Limit total and per-client concurrent waiters.
- [x] Bound event payload size and avoid retaining unbounded event history.
- [x] Release waiting requests cleanly during graceful shutdown.

**Done when:** a client can wait for an event, receive one after a matching
write, time out without an event, and cancel without leaving server state.

## 8. Observability and Operations

- [x] Add health and readiness checks that distinguish a running process from
  an available SQLite dependency.
- [x] Emit structured Tracing logs for request completion, database errors,
  cache behavior, and long-poll lifecycle events.
- [x] Expose metrics or a diagnostics endpoint for request count and latency,
  SQLite failures, cache counters, cache size, and active long-poll waiters.
- [x] Render diagnostics in an HTML dashboard served by the binary with plain
  JavaScript and no build tooling.
- [x] Document all configuration values, defaults, validation rules, and safe
  production recommendations.
- [x] Document SQL access policy, limits, and operational caveats of serving
  SQLite over HTTP.

**Done when:** an operator can identify request failures, cache effectiveness,
database unavailability, and leaked-or-excessive waiters from logs or metrics.

## 9. Python End-to-End Tests

- [ ] Choose Python test and HTTP-client libraries and document setup.
- [ ] Build fixtures that create a temporary database, start the binary on an
  available port, wait for readiness, and always tear down the process.
- [ ] Test successful parameterized raw-SQL reads and writes through
  `POST /api/query`, and valid Squeal selects through the same endpoint.
- [ ] Test `null`, numeric, string, and boundary parameter values.
- [ ] Test result columns, rows, write metadata, invalid JSON, invalid fields,
  mutually exclusive `sql`/`squeal` validation, Squeal validation, SQL policy
  rejection, and SQLite constraint failures.
- [ ] Test cache hits, parameter-sensitive keys, write invalidation, and
  mark-and-sweep behavior using observable counters or diagnostics.
- [ ] Test long-poll event delivery, timeout, malformed requests, concurrent
  waiters, disconnect cleanup, and shutdown behavior.
- [ ] Ensure all network waits have explicit, short test timeouts and avoid
  sleeps when synchronization signals are available.

**Done when:** the suite runs reliably from a clean checkout and exercises all
public feature requirements over real HTTP.

## 10. Release Readiness

- [ ] Run formatter, linter, Rust unit tests, and Python end-to-end tests in
  continuous integration.
- [ ] Add a security review of SQL policy, request-size limits, error output,
  authentication assumptions, and long-poll resource limits.
- [ ] Benchmark cache hit and miss behavior, write invalidation, and concurrent
  long-poll capacity against stated deployment expectations.
- [ ] Write a quick-start guide with local configuration and example `curl`
  requests.
- [ ] Review API and operational documentation for consistency with the
  shipped behavior.

**Done when:** CI is green, documentation matches the implementation, and the
service can be run and evaluated using the quick-start guide.
