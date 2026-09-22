#!/usr/bin/env bash
set -e
exec cargo fmt --all -- --check
