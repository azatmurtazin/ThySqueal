# Python End-to-End Testing

## Purpose

Python end-to-end tests verify ThySqueal as clients use it: through its actual
HTTP interface and a controlled SQLite database. They complement unit tests by
testing process startup, networking, serialization, and feature integration.

## Test Harness

- Each test suite creates an isolated temporary SQLite database.
- The harness starts ThySqueal with deterministic configuration and waits until
  it is ready to accept requests.
- Tests send real HTTP requests and parse JSON responses.
- Teardown stops the server and removes temporary data, even after failures.

## Setup and Layout

Prerequisites: a Rust toolchain to build the server binary, Python 3.10+, and
[`uv`](https://docs.astral.sh/uv/).

```sh
cargo build
uv sync
just test-e2e
```

The `test-e2e` recipe builds the binary and runs `uv run pytest tests`, which
creates a `.venv` from the `dev` dependency group declared in `pyproject.toml`
and a `uv.lock` lockfile. The `THYSQUEAL_BIN` environment variable overrides
the server binary location when running pytest directly. The GitHub Actions
workflow in `.github/workflows/ci.yml` runs the Rust checks and this suite on
push to `main` and on pull requests. The suite lives under `tests/`:

- `harness.py` starts and stops real server processes on ephemeral ports, seeds
  isolated SQLite databases before startup, writes the YAML configuration, and
  exposes `/readyz`-based readiness and `/api/diagnostics` signals.
- `conftest.py` provides `harness` (server factory), `server`, and `client`
  fixtures that always tear the process down.
- `test_query.py` exercises `POST /api/query`: parameterized reads and writes,
  Squeal selects, validation, and error mapping.
- `test_cache.py` verifies hit/miss counters, parameter-sensitive keys, write
  invalidation, and deterministic mark-and-sweep collection.
- `test_events.py` verifies long-poll delivery, timeout, validation, waiter
  limits, disconnect cleanup, and shutdown behavior.
- `test_ops.py` verifies `/healthz`, `/readyz`, diagnostics JSON, and the HTML
  dashboard.

## Required Coverage

- Parameterized raw-SQL reads and writes, nulls, numeric values, and strings.
- Valid Squeal selects, malformed Squeal objects, requests containing both
  `sql` and `squeal`, and `params` combined with `squeal`.
- Result metadata, row serialization, non-row statement responses, and JSON
  error bodies.
- SQLite constraint failures, unknown-database requests, and locked databases
  (an external exclusive lock makes writes fail with 503 `unavailable` until
  the lock is released).
- Cache hit behavior, parameter-sensitive keys, write invalidation, and
  mark-and-sweep removal of unused entries.
- Long-poll event delivery, timeout, malformed input, concurrent waiters, and
  client disconnect cleanup.

Parameter-count mismatches are not validated by the server: SQLite silently
binds `NULL` to unbound placeholders and ignores surplus bindings, so no
HTTP-level test asserts on them. The per-client waiter limit cannot be
exercised over HTTP because hyper serializes pipelined requests on a
connection, so every connection carries at most one active waiter; that limit
is enforced and tested at the Rust unit level instead.

## Test Design Principles

Tests must avoid timing assumptions where possible. Long-poll tests synchronize
on explicit readiness signals and use bounded timeouts. Cache assertions use
observable metrics, logs, or controlled database behavior rather than relying
only on elapsed time. Test data is local and self-contained.

## Acceptance Criteria

- The suite can run from a clean checkout with documented prerequisites.
- Tests are isolated and safe to run repeatedly or in parallel.
- Feature regressions surface through a failing HTTP-level test.
