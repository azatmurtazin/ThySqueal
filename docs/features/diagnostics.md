# Diagnostics and Observability

## Purpose

Operators need to see whether the server is healthy, whether requests are
failing, how effective the select cache is, and whether long-poll waiters are
bounded. ThySqueal exposes this information through a JSON diagnostics endpoint
and a small HTML dashboard that renders it.

## Implementation

A shared `Metrics` registry (atomics plus a latency histogram) records request
count, in-flight requests, response status buckets, request latency, SQLite
execution counts and failures, published change events, and long-poll
lifecycle counters. A Tower layer around the whole router counts every request
and records its status and latency when it completes; cancelled requests still
release the in-flight counter.

The `GET /api/diagnostics` handler aggregates the metric snapshot with live
state: per-database pool and cache sizes, cache counters, and current
long-poll waiter counts and limits.

The HTML dashboard at `GET /diagnostics` is a single static page served by the
binary. It fetches `GET /api/diagnostics` with plain JavaScript and re-renders
on an auto-refresh interval; styling uses a classless CSS theme loaded from a
CDN, so no Node.js or build tooling is involved.

## Endpoint: `GET /api/diagnostics`

Returns `200 OK` with `Content-Type: application/json`. No query parameters
are accepted.

```json
{
  "started_at_millis": 1710000000000,
  "uptime_seconds": 612.4,
  "requests": {
    "total": 48,
    "in_flight": 1,
    "responses_2xx": 40,
    "responses_3xx": 0,
    "responses_4xx": 7,
    "responses_5xx": 1,
    "latency": {
      "mean_ms": 12.3,
      "p50_ms": 5.0,
      "p90_ms": 100.0,
      "p99_ms": 2000.0,
      "max_ms": 4102.7
    }
  },
  "sqlite": {
    "executions": 31,
    "errors": 2
  },
  "long_poll": {
    "active": 0,
    "max": 1000,
    "max_per_client": 10,
    "waits": 6,
    "timeouts": 5,
    "shutdowns": 0,
    "rejected_total": 0,
    "rejected_per_client": 0,
    "events_published": 4
  },
  "databases": [
    {
      "name": "main",
      "pool_connections": 2,
      "pool_idle": 1,
      "cache_entries": 3,
      "cache_bytes": 412,
      "cache_max_entries": 1000,
      "counters": {
        "hits": 12,
        "misses": 6,
        "stores": 5,
        "invalidations": 2,
        "collection_runs": 0,
        "swept_entries": 0
      }
    }
  ]
}
```

Field meaning:

- `started_at_millis` — Unix milliseconds when the server started.
- `uptime_seconds` — seconds since start.
- `requests` — counters for completed requests. `total` is the number of
  completed requests; `in_flight` is the number currently being handled
  (including the diagnostics request itself). `responses_2xx`, `responses_3xx`,
  `responses_4xx`, and `responses_5xx` are status-code buckets.
- `requests.latency` — latency in milliseconds for completed requests. `p50`,
  `p90`, and `p99` are approximate percentiles derived from fixed latency
  buckets (`1ms`, `5ms`, `25ms`, `100ms`, `500ms`, `2s`, `10s`); `mean_ms` and
  `max_ms` are exact.
- `sqlite` — `executions` counts SQLite statements that ran, `errors` counts
  executions that failed. Requests answered entirely from the select cache do
  not increment `executions`.
- `long_poll` — `active` is the current number of waiting clients, `max` and
  `max_per_client` are the configured waiter limits. `waits`, `timeouts`,
  `shutdowns`, `rejected_total`, and `rejected_per_client` are lifecycle
  counters; `events_published` counts change events sent to the per-database
  channel.
- `databases` — one entry per configured database: pool connection counts,
  current cache size and configured maximum, and the cache counter snapshot.

The counters are monotonic since process start; the endpoint never resets
them.

## Dashboard: `GET /diagnostics`

Returns `200 OK` with `Content-Type: text/html`. The page shows request,
latency, SQLite, and long-poll summary cards plus a table of per-database pool
and cache state. It fetches `/api/diagnostics` on load and re-fetches on the
selected auto-refresh interval (default `2s`); a refresh control can pause
updates. When the endpoint is unreachable, the page shows an error banner
instead of stale numbers.

## Acceptance Criteria

- An operator can read request count and latency, SQLite failures, cache
  counters and size, and active long-poll waiters from `GET /api/diagnostics`.
- The HTML dashboard renders that JSON with no build tooling or runtime
  dependencies beyond a CSS CDN link.
- Request metrics record every completed request and never leak in-flight
  counts after cancellation.
