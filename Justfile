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

PYTHON := env_var_or_default("PYTHON", "python3")

test-e2e:
  cargo build
  {{PYTHON}} -m pytest tests
