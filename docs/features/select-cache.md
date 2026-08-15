# In-Memory Select Cache

## Purpose

The select cache lowers latency and SQLite load for recently repeated,
cacheable `SELECT` requests. It holds response data only in process memory and
never replaces SQLite as the source of truth.

## Implementation

A `DashMap` (`src/cache.rs`) holds concurrent cache entries. Each value stores
the immutable response payload behind an `Arc` plus metadata for the
mark-and-sweep collector: mark generation flag, estimated size in bytes, and
creation and last-access timestamps. The project intentionally does not use a
general-purpose eviction cache: its collector implements the specified
mark-and-sweep policy directly. On a store, the cache first runs a collection
when a configured threshold is reached, then inserts only if the entry count is
still below `max_entries`, so the cache stays bounded without evicting
arbitrarily.

Every database owns a separate cache instance, so entries, capacity, and
invalidation never leak across databases. The database's `cache` section
configures it; a `max_entries` value of `0` disables caching for that database,
and omitted fields inherit the global `cache` defaults.

## Eligibility and Keys

- Only a single, read-only `SELECT` statement (raw SQL or compiled Squeal) is a
  cache candidate. Multi-statement requests, statements containing any write,
  and policy-rejected statements are never cached.
- Non-deterministic queries bypass the cache. `src/policy.rs` rejects function
  names that SQLite evaluates at execution time: `random`, `randomblob`,
  `changes`, `total_changes`, `last_insert_rowid`, the date/time family
  (`strftime`, `date`, `time`, `datetime`, `julianday`, `unixepoch`), and the
  `CURRENT_*`/`LOCALTIME*` keyword forms.
- The cache key is a byte string built from the canonical compiled SQL and a
  type-preserving, length-framed serialization of every bound parameter
  (`cache::build_key`). Parameters are tagged by type, so `1`, `1.0`, `"1"`,
  `true`, and `null` never collide even though SQLite may coerce them. Keys are
  scoped to a single database's cache, which keeps the key independent of the
  database name.
- Because Squeal is compiled to SQL before lookup, an equivalent raw-SQL request
  and Squeal request share a cache entry.

On a cache hit, ThySqueal returns the stored result in the same response shape
as a database execution (`build_cached_response`). On a miss it executes
SQLite, returns the result, and stores it when eligible.

## Invalidation

Any successful request whose statement classes include a write invalidates all
cached reads for that database, so a write can never leave stale results
behind. Invalidation is scoped to the database that was written; other
databases keep their cache entries. This is the initial correctness-first
policy; more targeted invalidation may be added only when it demonstrably
preserves correctness.

## Counters and Configuration

Atomic counters track hits, misses, stores, invalidations, collection runs,
and swept entries. At startup, `database::open_all` resolves each database's
effective cache settings and wires them into its own `SelectCache`, spawning a
periodic collection task when a collection interval is configured.

## Mark-and-Sweep Collection

Each cache entry carries a mark flag. A hit or fresh store marks the entry;
collection clears marks on survivors and sweeps entries that remain unmarked,
so entries used during the current generation survive the cycle and unused
entries are reclaimed. Collection is triggered on a store when the entry-count
or byte-count threshold is reached, and additionally runs on a periodic timer
when `collection_interval_seconds` is set. Collection walks the `DashMap` with
a `retain` pass and never holds map references across an async boundary, so it
is safe against concurrent request read and write traffic. Every run records
its duration, entry count before and after, bytes reclaimed, and entries swept.

## Time-Based Expiry

Entries expire when their `created` timestamp exceeds the configured maximum
age (`max_age_seconds`), even if recently marked; expired entries become sweep
candidates alongside unmarked ones. Because `created` and `last_access` are
tracked on every entry, expiry needs no schema change. Age-based expiry never
replaces write invalidation, which remains the correctness mechanism; it only
bounds how long an entry can be served without a write.

## Acceptance Criteria

- Repeating an eligible `SELECT` returns a cache hit without another SQLite
  read.
- Writes never allow stale cached query results to be served.
- Recently used entries survive a collection cycle; unused entries are removed.
- An entry older than the configured maximum age is not served once collected.
- Cache size remains bounded by configuration.
