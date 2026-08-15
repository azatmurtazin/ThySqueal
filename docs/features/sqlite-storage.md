# SQLite Storage

## Purpose

SQLite is ThySqueal's persistent data store. It supplies SQL execution,
durability, transactions, and schema management while ThySqueal supplies the
remote JSON API and cache behavior.

## Implementation

SQLx provides asynchronous SQLite access through a shared `SqlitePool`. SQLx
prepared queries bind request values without string interpolation and provide
dynamic rows and column metadata for the JSON API. The pool is created once at
startup and closed as part of graceful shutdown.

## Database Lifecycle

- The server opens a configured SQLite database when it starts.
- Startup fails clearly if the database cannot be opened or initialized.
- The database location and SQLite connection settings are configuration, not
  request input.
- SQLite pragmas and journaling mode are configured deliberately for the
  deployment rather than relying on implicit defaults.

## Execution Semantics

- SQL parameters are passed through SQLite's binding interface.
- Statements execute against the configured database in request order according
  to the server's concurrency model.
- SQLite transaction statements are honored when the configured access policy
  allows them.
- Reads return rows and column metadata. Writes report execution metadata and
  trigger cache invalidation.

## Access Policy

The server must define which statement classes it permits. At minimum, the
policy distinguishes read-only `SELECT` statements from data-changing
statements such as `INSERT`, `UPDATE`, and `DELETE`. Administrative statements
and SQLite extension loading should be disabled unless explicitly enabled by a
trusted deployment configuration.

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
