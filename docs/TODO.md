# Implementation TODO

This checklist turns the product requirements into an incremental implementation
plan. Complete each milestone with its verification steps before proceeding to
the next one.

## 1. Project Foundation

- [ ] Add the selected Rust dependencies: Axum, Tokio, Serde, `serde_json`,
  SQLx with SQLite support, Tower, `tower-http`, Tracing,
  `tracing-subscriber`, DashMap, and `thiserror`.
- [ ] Configure only the required crate features: Tokio runtime, networking,
  synchronization, timers, and signal handling; SQLx Tokio and SQLite support;
  and Tower HTTP tracing, request-ID, panic-catching, timeout, and size-limit
  layers.
- [ ] Define the executable entry point and a configuration model for database
  path, bind address, request limits, cache limits, and long-poll timeout.
- [ ] Add structured logging with `tracing`, request IDs from `tower-http`, and
  `tracing-subscriber` environment-filtered log levels.
- [ ] Define a server lifecycle: load configuration, open SQLite, initialize
  shared state, listen for requests, and gracefully shut down.
- [ ] Add development commands for formatting, linting, unit tests, and running
  the server locally.

**Done when:** the server starts with an empty SQLite database, exposes a
health or readiness endpoint, and exits cleanly on shutdown.

## 2. SQLite Integration

- [ ] Open a SQLx `SqlitePool` during startup and surface failures as clear
  startup errors.
- [ ] Set deliberate SQLite connection options, busy timeout, and pragmas.
- [ ] Create an execution module using SQLx prepared queries that accepts raw
  SQL plus bound values or compiled Squeal, then returns rows, column metadata,
  and write metadata.
- [ ] Implement a Squeal parser, validator, and compiler that converts its JSON
  AST into SQLite SQL plus bound values; do not accept arbitrary SQL fragments
  within Squeal fields.
- [ ] Model supported JSON values and convert them safely to and from SQLite
  values, including `null`, booleans, integers, floats, strings, and blobs if
  blobs are part of the public API.
- [ ] Define allowed raw-SQL statement classes and Squeal operations, and
  reject prohibited administrative or extension-loading statements.
- [ ] Map SQLite syntax, binding, constraint, busy, and internal errors to
  application error types without leaking database paths or internals.
- [ ] Ensure statements, transactions, and connections are released on all
  success, error, timeout, and cancellation paths.

**Done when:** a Rust-level test can create a table, insert bound values,
query rows, and receive correctly typed execution results.

## 3. JSON API: `POST /api/query`

- [ ] Set up Axum routes, extractors, application state, and Serde JSON
  request/response types.
- [ ] Implement request validation requiring exactly one of a non-empty `sql`
  string or a `squeal` object; accept `params` only with raw `sql`.
- [ ] Bind raw `params` and Squeal literal values through SQLite's parameter
  API; never construct SQL by interpolating client data.
- [ ] Implement the response envelope with `meta` and `rows` for every
  successful statement.
- [ ] Return column names and row count for row-producing statements.
- [ ] Return affected-row count and last inserted row ID where meaningful for
  non-row statements, with `rows: []`.
- [ ] Define stable JSON error objects containing a machine-readable code and a
  safe, client-useful message.
- [ ] Map invalid JSON and validation errors to `400`, rejected SQL policy to
  `422`, unavailable database to `503`, and unexpected failures to `500`.
- [ ] Set response content type and add Tower request body-size and request
  timeout limits.

**Done when:** a client can execute parameterized raw `SELECT`, `INSERT`, and
`UPDATE`, plus a valid Squeal `select`, and invalid requests through HTTP and
receive documented responses.

## 4. Query Classification and Cache Boundary

- [ ] Classify raw SQL and compiled Squeal statements as cacheable read,
  data-changing write, or uncached operation using a conservative policy.
- [ ] Initially cache only unambiguous, read-only `SELECT` statements.
- [ ] Explicitly bypass caching for non-deterministic or unsupported queries.
- [ ] Define a canonical cache key from compiled SQL and a type-preserving
  serialization of all bound parameters, independent of whether the client
  used raw SQL or Squeal.
- [ ] Define immutable cached result data matching the HTTP response envelope.
- [ ] Store cache entries in DashMap and keep cache operations safe under
  concurrent requests.

**Done when:** logically distinct requests, including requests whose parameters
have different values or types, cannot share a cache entry.

## 5. In-Memory Select Cache

- [ ] Implement DashMap cache lookup before SQLx execution for cacheable reads.
- [ ] Store successful eligible read results after SQLite execution.
- [ ] Track entry size, creation time, last access, and mark state.
- [ ] Invalidate all cached selects after each successful write as the initial
  correctness-first policy.
- [ ] Add counters for hits, misses, stores, invalidations, collection runs,
  and swept entries.
- [ ] Add configuration for maximum entries and/or maximum bytes, collection
  threshold, and optional collection interval.

**Done when:** repeated eligible reads avoid a second database execution and a
successful write prevents any stale cached result from being served.

## 6. Mark-and-Sweep Cache Collection

- [ ] Mark cache entries on cache hits and on insertion when appropriate for
  the collection policy.
- [ ] Trigger collection when configured memory or entry thresholds are met,
  and optionally on a periodic timer.
- [ ] Sweep entries that were not marked in the current collection generation.
- [ ] Clear or advance marks on surviving entries so future collection cycles
  can distinguish recent use from old use.
- [ ] Make collection safe when requests read or write cache entries
  concurrently, using DashMap entry operations without holding references
  across async await points.
- [ ] Record collection duration, entry count before and after, bytes reclaimed,
  and number of entries swept.

**Done when:** tests show that accessed entries survive one collection cycle,
unused entries are reclaimed, and configured cache limits remain bounded.

## 7. Long Polling

- [ ] Define the long-poll endpoint, request filter, response schema, timeout
  status, and event schema in the public API documentation.
- [ ] Implement a registry of pending waiters with cancellation-safe cleanup.
- [ ] Use a Tokio `broadcast` channel to publish change events to concurrent
  waiters, applying `tokio::time::timeout` to each wait.
- [ ] Publish change events only after successful relevant writes commit.
- [ ] Deliver matching events to waiting requests and remove completed waiters.
- [ ] Implement a configurable maximum wait duration and normal timeout
  response when no event arrives.
- [ ] Detect client disconnects and unregister their waiters promptly.
- [ ] Limit total and per-client concurrent waiters.
- [ ] Bound event payload size and avoid retaining unbounded event history.
- [ ] Release waiting requests cleanly during graceful shutdown.

**Done when:** a client can wait for an event, receive one after a matching
write, time out without an event, and cancel without leaving server state.

## 8. Observability and Operations

- [ ] Add health and readiness checks that distinguish a running process from
  an available SQLite dependency.
- [ ] Emit structured Tracing logs for request completion, database errors,
  cache behavior, and long-poll lifecycle events.
- [ ] Expose metrics or a diagnostics endpoint for request count and latency,
  SQLite failures, cache counters, cache size, and active long-poll waiters.
- [ ] Document all configuration values, defaults, validation rules, and safe
  production recommendations.
- [ ] Document SQL access policy, limits, and operational caveats of serving
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
