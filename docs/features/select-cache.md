# In-Memory Select Cache

## Purpose

The select cache lowers latency and SQLite load for recently repeated,
cacheable `SELECT` requests. It holds response data only in process memory and
never replaces SQLite as the source of truth.

## Implementation

A `DashMap` (`src/cache.rs`) holds concurrent cache entries. Each value stores
the immutable response payload behind an `Arc` plus metadata for the future
mark-and-sweep collector: mark generation flag, estimated size in bytes, and
creation and last-access timestamps. The project intentionally does not use a
general-purpose eviction cache: its collector must implement the specified
mark-and-sweep policy directly. Until that collector lands, `store` simply
skips new entries once the configured `max_entries` count is reached, so the
cache stays bounded without evicting anything.

## Eligibility and Keys

- Only a single, read-only `SELECT` statement (raw SQL or compiled Squeal) is a
  cache candidate. Multi-statement requests, statements containing any write,
  and policy-rejected statements are never cached.
- Non-deterministic queries bypass the cache. `src/policy.rs` rejects function
  names that SQLite evaluates at execution time: `random`, `randomblob`,
  `changes`, `total_changes`, `last_insert_rowid`, the date/time family
  (`strftime`, `date`, `time`, `datetime`, `julianday`, `unixepoch`), and the
  `CURRENT_*`/`LOCALTIME*` keyword forms.
- The cache key is a byte string built from the database name, the canonical
  compiled SQL, and a type-preserving, length-framed serialization of every
  bound parameter (`cache::build_key`). Parameters are tagged by type, so
  `1`, `1.0`, `"1"`, `true`, and `null` never collide even though SQLite may
  coerce them.
- Because Squeal is compiled to SQL before lookup, an equivalent raw-SQL request
  and Squeal request share a cache entry.

On a cache hit, ThySqueal returns the stored result in the same response shape
as a database execution (`build_cached_response`). On a miss it executes
SQLite, returns the result, and stores it when eligible.

## Invalidation

Any successful request whose statement classes include a write invalidates all
cached reads, so a write can never leave stale results behind. This is the
initial correctness-first policy; more targeted invalidation may be added only
when it demonstrably preserves correctness.

## Counters and Configuration

Atomic counters track hits, misses, stores, invalidations, collection runs,
and swept entries (`collection_runs` and `swept_entries` are reserved for the
mark-and-sweep collector). The server wires `cache_max_entries` from the
configuration into `SelectCache::new` at startup.

## Mark-and-Sweep Collection

Planned for a later milestone. Each cache entry already tracks whether it was
used since the last collection cycle: a hit or fresh store updates the mark and
last-access time. The future collector will sweep entries that remain unmarked
and clear marks on retained entries for the next cycle.

## Acceptance Criteria

- Repeating an eligible `SELECT` returns a cache hit without another SQLite
  read.
- Writes never allow stale cached query results to be served.
- Recently used entries survive a collection cycle; unused entries are removed.
- Cache size remains bounded by configuration.
