# Product Requirements Document: ThySqueal

## Overview

ThySqueal is a lightweight JSON-over-HTTP API server for querying a SQLite
database with either raw SQL or Squeal, a structured JSON representation of
SQL. It provides a small, predictable interface for applications that need
database access without managing a direct SQLite connection.

## Technology Stack

- Rust is the implementation language.
- Axum and Tokio provide the asynchronous HTTP server and request lifecycle.
- Serde and `serde_json` serialize API payloads.
- SQLx provides asynchronous SQLite access and connection pooling.
- Tower and `tower-http` provide request limits, timeouts, panic handling,
  request IDs, and tracing middleware.
- DashMap stores the custom concurrent mark-and-sweep select-query cache.
- Tracing and `tracing-subscriber` provide structured application logs.

## Goals

- Expose SQLite through a JSON API.
- Support parameterized raw SQL statements and structured Squeal queries.
- Return query results in a consistent, machine-readable shape.
- Reduce repeated read-query latency with an in-memory cache.
- Support long-polling clients for connections that need to wait for updates.
- Validate the server end to end with Python tests.

## API

### Execute a Query

`POST /api/query` executes one raw SQL statement or one Squeal query.

Raw SQL request:

```json
{
  "sql": "select * from posts",
  "params": []
}
```

`params` is optional and contains values bound to the SQL statement. The API
must use parameter binding rather than interpolating values into SQL text.

`db` is an optional string selecting one of the configured named SQLite
databases; when omitted it defaults to `main`.

Squeal request:

```json
{
  "squeal": {
    "_": "select",
    "from": "posts",
    "cols": ["*"]
  }
}
```

The request body contains exactly one of `sql` or `squeal`. Squeal is compiled
by the server to parameterized SQLite SQL; literal values in a Squeal expression
are bound internally and must not be concatenated into generated SQL text.

Successful response:

```json
{
  "meta": {
    "columns": ["id", "title"]
  },
  "rows": [
    { "id": 1, "title": "First post" }
  ]
}
```

`meta` contains information about the execution and result set, including
column metadata when rows are returned. `rows` contains result records; it is
an empty array for statements that produce no rows. Errors must use a clear
JSON error response with a stable, machine-readable code and an appropriate
HTTP status code: `400` for invalid requests, `503` when the database cannot
serve requests, and `500` for unexpected failures.

## Data Storage

- SQLite is the underlying persistent data store. The server can be configured
  to expose multiple named SQLite databases.
- The server supports raw SQL statements and Squeal operations permitted by its
  configured database and access policy.
- Write operations must invalidate affected cached read results so subsequent
  reads do not return stale data.

## Select Query Cache

- Recent `SELECT` queries from either query representation are cached in memory.
- A cache key is based on canonical compiled SQL and bound parameters, so
  semantically equivalent Squeal and raw SQL requests can share an entry while
  different parameter values remain independent.
- Repeated eligible `SELECT` requests may return the cached response.
- Cache memory is reclaimed using mark-and-sweep garbage collection: recently
  used entries are marked, and unmarked entries are swept during collection.
- Cache entries may also be reclaimed after a configurable maximum age (TTL),
  in addition to write invalidation and mark-and-sweep collection.
- Cache limits and collection behavior should be configurable and observable
  through server metrics or logs.

## Long Polling

- The server supports long-polling connections for clients waiting on database
  changes or other supported events.
- A long-poll request remains open until an event is available or a configured
  timeout is reached.
- Timeouts, client disconnects, and malformed requests must be handled without
  leaking connections or memory.

## Testing

- End-to-end tests are written in Python.
- Tests start the server against a controlled SQLite database and exercise the
  HTTP interface.
- Coverage includes parameter binding, successful reads and writes, response
  shape, error handling, cache hits and invalidation, cache collection, and
  long-poll success, timeout, and disconnect behavior.

## Non-Goals

- Replacing a full relational database server.
- Providing an ORM, schema migration system, or SQL dialect beyond SQLite.
- Supporting arbitrary non-JSON request or response formats.
