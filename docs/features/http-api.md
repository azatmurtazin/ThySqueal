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
invalid. Squeal values are compiled to SQL with bound values, so client data
never becomes generated SQL text.

```json
{
  "squeal": {
    "_": "select",
    "from": "posts",
    "cols": ["id", "title"]
  }
}
```

Squeal is recognized by the server but is not yet supported; such requests
currently fail with a `squeal_unsupported` error until the Squeal compiler
lands. See the [Squeal query language](squeal.md) for the structured-language
contract. `params` is accepted only with raw `sql`; Squeal encodes its values
in the Squeal object.

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

Error codes are stable and machine-readable. The full mapping:

| HTTP status | Error code | Condition |
| --- | --- | --- |
| `400` | `invalid_request` | Invalid JSON, missing or conflicting fields, empty `sql`, invalid parameter shapes. |
| `400` | `squeal_unsupported` | A `squeal` field was supplied before the Squeal compiler is available. |
| `400` | `unknown_database` | The `db` field names a database that is not configured. |
| `400` | `invalid_sql` | SQL syntax, `no such table`, or `no such column` failures. |
| `400` | `constraint_violation` | A `UNIQUE`, `NOT NULL`, `PRIMARY KEY`, `FOREIGN KEY`, or `CHECK` constraint failed. |
| `400` | `unsupported_column` | A result column has a type the value model cannot represent, such as a blob. |
| `503` | `unavailable` | The database is locked or the pool cannot serve requests. |
| `500` | `execution_failed` | Any unexpected execution failure. |

## Acceptance Criteria

- A client can execute allowed raw SQL queries through `POST /api/query`.
- Parameter values never change SQL syntax through string interpolation.
- Responses have a consistent JSON shape for both row and non-row statements.
- Invalid requests and execution failures provide safe JSON error responses.
