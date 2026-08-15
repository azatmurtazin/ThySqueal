# HTTP SQL API

## Purpose

ThySqueal exposes SQLite operations through a JSON HTTP interface. The API is
intended for clients that need a simple remote SQL boundary and a stable,
machine-readable response format.

## Implementation

Axum supplies the router, `POST /squeal` handler, JSON extractors, and shared
application state. Serde and `serde_json` define and serialize the request,
response, and error types. Tower and `tower-http` enforce request-size and
timeout limits, attach request IDs, catch panics, and emit HTTP tracing data.

## Endpoint

`POST /squeal` accepts one SQL statement and its bound parameters.

Requests must use `Content-Type: application/json`.

```json
{
  "sql": "SELECT id, title FROM posts WHERE author_id = ?",
  "params": [42]
}
```

`sql` is required and must be a non-empty string. `params` is optional; when
omitted, it behaves as an empty parameter list. Values are bound by SQLite,
not interpolated into the SQL string.

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
    "message": "sql must be a non-empty string"
  }
}
```

Use `400 Bad Request` for invalid JSON, missing or invalid fields, and invalid
parameter shapes; `422 Unprocessable Content` for SQL rejected by policy;
`500 Internal Server Error` for unexpected failures; and `503 Service
Unavailable` when the database cannot serve requests.

## Acceptance Criteria

- A client can execute parameterized reads and writes through `POST /squeal`.
- Parameter values never change SQL syntax through string interpolation.
- Responses have a consistent JSON shape for both row and non-row statements.
- Invalid requests and execution failures provide safe JSON error responses.
