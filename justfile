default:
    @just --list

format:
    cargo fmt --all

format-check:
    cargo fmt --all -- --check

lint:
    cargo clippy -- -D warnings

build:
    cargo build

test:
    cargo test

typos:
    typos

CONTAINER_RUNNER := env("CONTAINER_RUNNER", "podman")

ci: format-check lint build test typos shell-test test-all-containers

shell-test:
    {{ if os() == "windows" { "powershell -File ./tests/shell_test.ps1 ./target/debug/asleep.exe" } else { "./tests/shell_test.sh ./target/debug/asleep" } }}

test-all-containers: test-debian test-alpine

test-debian:
    {{ CONTAINER_RUNNER }} build -t asleep-test-debian -f Containerfile .
    {{ CONTAINER_RUNNER }} run --rm asleep-test-debian

test-alpine:
    {{ CONTAINER_RUNNER }} build -t asleep-test-alpine -f Containerfile.alpine .
    {{ CONTAINER_RUNNER }} run --rm asleep-test-alpine
