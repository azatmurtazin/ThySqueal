default:
  just --list

build-dev:
  cargo build

run-dev:
  ./target/debug/thy-squeal
