#!/usr/bin/env bash
set -e
exec cargo test --lib --all-features --no-fail-fast
