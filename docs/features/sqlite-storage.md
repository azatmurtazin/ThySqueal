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
  SQLite's parameter interface, never interpolated into the SQL text. The
  planned access policy will narrow which statement classes are allowed.
- SQLite transaction statements are honored when the configured access policy
  allows them.
- Reads return rows and column metadata. Writes report execution metadata and
  trigger cache invalidation.

## Access Policy

The server must define which statement classes it permits. At minimum, the
  policy distinguishes read-only `SELECT` statements from data-changing
  statements such as `INSERT`, `UPDATE`, and `DELETE`, including their Squeal
  equivalents when supported. Administrative statements and SQLite extension
  loading should be disabled unless explicitly enabled by a trusted deployment
  configuration.

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
