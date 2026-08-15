# SQLite Storage

## Purpose

SQLite is ThySqueal's persistent data store. It supplies SQL execution,
durability, transactions, and schema management while ThySqueal supplies the
remote JSON API and cache behavior.

## Implementation

SQLx provides asynchronous SQLite access through a shared `SqlitePool`. Raw
SQL parameters and values compiled from Squeal are bound through SQLx prepared
queries rather than string interpolation. SQLx provides dynamic rows and column
metadata for the JSON API. The pool is created once at startup and closed as
part of graceful shutdown.

## Database Lifecycle

- The server opens every configured SQLite database when it starts.
- Startup fails clearly if any database cannot be opened or initialized.
- The set of databases, their locations, and SQLite connection settings are
  configuration, not request input.
- SQLite pragmas and journaling mode are configured deliberately for the
  deployment rather than relying on implicit defaults.

## Execution Semantics

- Raw SQL parameters and Squeal literal values are passed through SQLite's
  binding interface.
- The value model covers `null`, booleans, integers, floats, and strings.
  Booleans bind and decode as the SQLite integers `0` and `1`. Blobs are not
  part of the public value model; a query returning a blob column fails as an
  unsupported column type.
- Squeal is validated and compiled into SQLite SQL before execution; identifiers
  are validated as language tokens and are never accepted as raw SQL fragments.
- Statements execute against the configured database in request order according
  to the server's concurrency model.
- Raw SQL may contain multiple statements; all values are still bound through
  SQLite's parameter interface, never interpolated into the SQL text. The access
  policy allows only read-only and data-changing statement classes; see
  Access Policy below.
- Transaction control statements are rejected by the access policy.
- Reads return rows and column metadata. Writes report execution metadata and
  trigger cache invalidation.

## Access Policy

Raw SQL is classified before execution. Each statement is tokenized to find
statement boundaries and the leading operation; strings, quoted identifiers,
comments, numbers, and other syntax are skipped so they cannot influence
classification.

- Read-only statements: `SELECT`, and `WITH` queries whose body selects data.
- Data-changing statements: `INSERT`, `UPDATE`, `DELETE`, `REPLACE`, and `WITH`
  queries whose body changes data.
- All other statement classes — DDL such as `CREATE`, `DROP`, and `ALTER`,
  transaction control (`BEGIN`, `COMMIT`, `ROLLBACK`), `PRAGMA`, `ATTACH`,
  `DETACH`, `VACUUM`, `EXPLAIN`, and anything unrecognized — are rejected with
  `422 Unprocessable Content` and the error code `policy_rejection`.
- Classification is fail-closed: statements that cannot be recognized are
  rejected rather than executed.
- Squeal compiles to `SELECT` statements, so it always classifies as read-only.
- SQLite extension loading is disabled at the connection level; the policy does
  not rely on keyword detection to block `SELECT load_extension(...)`.

## Failure Handling

SQLite constraint, syntax, and binding failures are translated to API errors.
Database-busy and locked conditions must be bounded by configured timeout or
retry behavior, then returned as a service-availability error. Connections and
statement resources are released on every success, failure, and client
disconnect path.

## Acceptance Criteria

- Data written through the API persists after a server restart.
- Bound values are handled by SQLite without SQL-text interpolation.
- SQLite errors produce safe, useful API failures.
- Data-changing statements notify the cache layer before later reads are
  served.
