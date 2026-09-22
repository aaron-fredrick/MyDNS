#!/usr/bin/env bash
set -e
cargo clippy --all-targets --all-features -- -D warnings
exec cargo check --all-targets --all-features
