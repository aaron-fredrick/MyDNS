#!/usr/bin/env bash
set -e
cargo fmt --all -- --check
exec cargo clippy --all-targets --all-features -- -D warnings
