# Repository Guide

## Project

ThySqueal is a Rust JSON-over-HTTP server for SQLite. The primary query endpoint
is `POST /api/query`; a request contains exactly one of raw `sql` or structured
`squeal`. Consult [docs/PRD.md](docs/PRD.md) and the feature documents in
`docs/features/` before changing public behavior.

## Implementation Conventions

- Use Axum and Tokio for HTTP and asynchronous request handling.
- Use Serde and `serde_json` for API types.
- Use SQLx with SQLite and bind all values; never interpolate client-controlled
  data into SQL.
- Compile and validate Squeal before executing or caching it. Squeal identifiers
  must never be treated as arbitrary SQL fragments.
- Implement the select cache with DashMap and the documented mark-and-sweep
  policy; do not replace it with a general-purpose eviction cache.
- Preserve cache correctness: invalidate cached reads after successful writes.
- Keep long-poll waiters bounded and cancellation-safe.

## Validation

Run these commands before handing off Rust changes:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

The same checks run through `.pre-commit-config.yaml`. Python end-to-end tests
must exercise the running HTTP server and use isolated temporary SQLite
databases when that suite is added.

## Documentation

- Update `docs/PRD.md` and the relevant file in `docs/features/` for public API
  or behavioral changes.
- Update `docs/TODO.md` when implementation milestones materially change.
- Keep README links accurate when adding or renaming documentation.

## Git Hygiene

- Keep commits focused and use Conventional Commit-style prefixes such as
  `feat:`, `fix:`, `docs:`, `test:`, or `ci:`.
- Do not commit generated `target/` content, local databases, credentials, or
  secrets.
