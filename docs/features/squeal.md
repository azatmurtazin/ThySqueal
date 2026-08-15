# Squeal Query Language

## Purpose

Squeal is ThySqueal's JSON representation of SQL. It lets clients express a
query as structured data instead of composing a raw SQL string. A request uses
either Squeal or raw SQL, never both.

## Initial Form

The initial supported operation is `select`:

```json
{
  "_": "select",
  "from": "posts",
  "cols": ["*"]
}
```

`_` identifies the operation, `from` is a table identifier, and `cols` is a
non-empty list of column identifiers or `"*"`. The server validates all
identifiers against Squeal's identifier grammar before compiling the object to
SQLite SQL. It must never treat a string from a Squeal field as an arbitrary
SQL fragment.

## Request Contract

Squeal is supplied as the `squeal` field of `POST /api/query`:

```json
{
  "squeal": {
    "_": "select",
    "from": "posts",
    "cols": ["id", "title"]
  }
}
```

The request must not contain `sql` or `params` when it contains `squeal`.
Squeal literals introduced by future operations are converted to bound SQLite
parameters by the compiler.

## Compilation and Errors

The compiler transforms a valid Squeal expression into SQLite SQL and an ordered
parameter list, then sends both to SQLx. Compilation happens before statement
classification and cache lookup. Invalid operation names, field types,
identifier syntax, and unsupported expression shapes produce a `400 Bad
Request` JSON error; Squeal operations rejected by the SQL access policy produce
`422 Unprocessable Content`.

## Extensibility

Additional operations and clauses may be added as explicit Squeal forms. Each
form must document its JSON schema, SQLite compilation rules, bound-value
behavior, cache eligibility, and access-policy classification before it is
enabled.

## Acceptance Criteria

- The example select object compiles to a read-only SQLite query.
- Squeal and raw SQL are mutually exclusive request forms.
- Client-controlled values use bound parameters; client-controlled identifiers
  are validated rather than interpolated as arbitrary SQL.
- Invalid Squeal returns a safe, stable JSON error.
