#!/usr/bin/env bash
set -e

npm ci
npm run build

rm -rf out/web
mkdir -p out
mv src/web/dist out/web

exec cargo test --lib --all-features --no-fail-fast
