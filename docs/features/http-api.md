# HTTP Query API

## Purpose

ThySqueal exposes SQLite operations through a JSON HTTP interface. A request
uses either raw SQL or Squeal, a structured JSON representation of SQL. The API
is intended for clients that need a simple remote database boundary and a
stable, machine-readable response format.

## Implementation

Axum supplies the router, `POST /api/query` handler, JSON extractors, and shared
application state. Serde and `serde_json` define and serialize the request,
response, and error types. Tower and `tower-http` enforce request-size and
timeout limits, attach request IDs, catch panics, and emit HTTP tracing data.

## Endpoint

`POST /api/query` accepts exactly one raw SQL statement or Squeal query.

Requests must use `Content-Type: application/json`.

`db` is an optional string naming the configured SQLite database to execute
against; when omitted it defaults to `main`. An unknown database name is a
client error.

### Raw SQL

`sql` must be a non-empty string. `params` is optional; when omitted, it
behaves as an empty parameter list. Values are bound by SQLite, not
interpolated into the SQL string.

```json
{
  "db": "main",
  "sql": "SELECT id, title FROM posts WHERE author_id = ?",
  "params": [42]
}
```

### Squeal

`squeal` must be a JSON object describing one query. It is an alternative to,
not an addition to, `sql`; requests containing both fields or neither field are
invalid. Squeal is compiled into parameterized SQLite SQL before execution;
Squeal values and identifiers are validated, so client data never becomes
arbitrary SQL text.

```json
{
  "squeal": {
    "_": "select",
    "from": "posts",
    "cols": ["id", "title"]
  }
}
```

See the [Squeal query language](squeal.md) for the structured-language
contract, including its identifier grammar. `params` is accepted only with raw
`sql`; Squeal encodes its values in the Squeal object.

## Successful Responses

Every successful response is JSON with `meta` and `rows` fields.

```json
{
  "meta": {
    "columns": ["id", "title"],
    "row_count": 1
  },
  "rows": [
    { "id": 7, "title": "Hello, ThySqueal" }
  ]
}
```

For row-producing statements, `columns` contains result-column names in their
returned order and each entry in `rows` is an object keyed by column name. For
statements that do not produce rows, `rows` is `[]`; `meta` reports applicable
execution details such as affected-row count and last inserted row ID.

## Errors

Errors must be JSON and must not expose internal stack traces, database paths,
or secrets. Their body has a stable structure:

```json
{
  "error": {
    "code": "invalid_request",
    "message": "provide exactly one of sql or squeal"
  }
}
```

Use `400 Bad Request` for invalid JSON, missing or invalid fields, and invalid
parameter shapes; `422 Unprocessable Content` for SQL rejected by policy;
`500 Internal Server Error` for unexpected failures; and `503 Service
Unavailable` when the database cannot serve requests.

## Long-Poll Events: `GET /api/events`

`GET /api/events` long-polls for database-change events. It accepts the optional
query parameters `db` (default `main`), `table` (no table filter by default),
and `limit` (default `1`, between `1` and `100`). The connection remains open
until a matching event is published, the configured timeout elapses, or the
server shuts down.

```http
GET /api/events?db=main&table=items&limit=1
```

A successful response carries a `meta` object naming the watched database and an
`events` array of change events; each event has a `database`, a `table` (or
`null` when the write's target table cannot be determined), and an `at` Unix
millisecond timestamp:

```json
{
  "meta": { "database": "main" },
  "events": [
    { "database": "main", "table": "items", "at": 1710000000000 }
  ]
}
```

Events are published only after successful writes commit, through a bounded
per-database channel. See [Long Polling](long-polling.md) for the full contract,
including filter validation and waiter limits.

Error codes are stable and machine-readable. The full mapping:

| HTTP status | Error code | Condition |
| --- | --- | --- |
| `400` | `invalid_request` | Invalid JSON, missing or conflicting fields, empty `sql`, invalid parameter shapes. |
| `400` | `invalid_squeal` | Invalid or unsupported Squeal: unknown operation, invalid field types, or identifier-syntax errors. |
| `400` | `unknown_database` | The `db` field names a database that is not configured. |
| `400` | `invalid_sql` | SQL syntax, `no such table`, or `no such column` failures. |
| `400` | `constraint_violation` | A `UNIQUE`, `NOT NULL`, `PRIMARY KEY`, `FOREIGN KEY`, or `CHECK` constraint failed. |
| `400` | `unsupported_column` | A result column has a type the value model cannot represent, such as a blob. |
| `422` | `policy_rejection` | Raw SQL uses a statement class outside the allowed read-only and data-changing set, such as DDL, `PRAGMA`, or transaction control. |
| `503` | `unavailable` | The database is locked or the pool cannot serve requests. |
| `500` | `execution_failed` | Any unexpected execution failure. |

The long-poll endpoint has additional error codes: `408 long_poll_timeout` when
no matching event arrives within the configured wait, `429 too_many_waiters`
when a client exceeds its per-client waiter limit, `503 too_many_waiters` when
the total waiter limit is reached, and `503 shutting_down` during graceful
shutdown.

## Acceptance Criteria

- A client can execute allowed raw SQL and Squeal queries through
  `POST /api/query`.
- Parameter values never change SQL syntax through string interpolation.
- Responses have a consistent JSON shape for both row and non-row statements.
- Invalid requests and execution failures provide safe JSON error responses.
