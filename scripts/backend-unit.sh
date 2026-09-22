#!/usr/bin/env bash
set -e

npm --prefix src/frontend ci
npm --prefix src/frontend run build

exec cargo test --lib --all-features --no-fail-fast
