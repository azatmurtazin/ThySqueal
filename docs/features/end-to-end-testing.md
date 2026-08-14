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

## Required Coverage

- Parameterized reads, writes, nulls, numeric values, strings, and invalid
  parameter counts.
- Result metadata, row serialization, non-row statement responses, and JSON
  error bodies.
- SQLite constraint and unavailable/locked-database failure paths.
- Cache hit behavior, parameter-sensitive keys, write invalidation, and
  mark-and-sweep removal of unused entries.
- Long-poll event delivery, timeout, malformed input, concurrent waiters, and
  client disconnect cleanup.

## Test Design Principles

Tests must avoid timing assumptions where possible. Long-poll tests synchronize
on explicit readiness signals and use bounded timeouts. Cache assertions use
observable metrics, logs, or controlled database behavior rather than relying
only on elapsed time. Test data is local and self-contained.

## Acceptance Criteria

- The suite can run from a clean checkout with documented prerequisites.
- Tests are isolated and safe to run repeatedly or in parallel.
- Feature regressions surface through a failing HTTP-level test.
