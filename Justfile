default:
  just --list

build-dev:
  cargo build

run-dev:
  ./target/debug/thy-squeal

fmt:
  cargo fmt --check

lint:
  cargo clippy --all-targets -- -D warnings

test:
  cargo test --all-targets

test-e2e:
  cargo build
  uv run pytest tests
