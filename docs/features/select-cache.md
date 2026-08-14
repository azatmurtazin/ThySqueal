# In-Memory Select Cache

## Purpose

The select cache lowers latency and SQLite load for recently repeated,
cacheable `SELECT` requests. It holds response data only in process memory and
never replaces SQLite as the source of truth.

## Eligibility and Keys

- Only read-only `SELECT` statements are cache candidates.
- The cache key includes a canonical SQL representation and a canonical,
  type-preserving representation of bound parameters.
- Requests with different parameter values or types must not share a key.
- Non-deterministic or policy-excluded statements bypass the cache.

On a cache hit, ThySqueal returns the stored result in the same response shape
as a database execution. On a miss, it executes SQLite, returns the result,
and stores it if eligible.

## Invalidation

Any successful data-changing statement invalidates cached reads that might no
longer be correct. The initial safe policy is to invalidate all select-cache
entries after every successful write. More targeted invalidation may be added
only when it demonstrably preserves correctness.

## Mark-and-Sweep Collection

Each cache entry tracks whether it was used since the last collection cycle.
Accessing an entry marks it. When configured cache pressure or a periodic
collection threshold is reached, the collector sweeps entries that remain
unmarked and clears marks on retained entries for the next cycle.

The cache configuration defines a maximum entry count and/or memory budget,
collection trigger, and observability level. Metrics or logs record hits,
misses, invalidations, collection runs, and entries swept.

## Acceptance Criteria

- Repeating an eligible `SELECT` returns a cache hit without another SQLite
  read.
- Writes never allow stale cached query results to be served.
- Recently used entries survive a collection cycle; unused entries are removed.
- Cache size remains bounded by configuration.
