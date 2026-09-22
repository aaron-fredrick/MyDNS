$ErrorActionPreference = "Stop"
cargo test --lib --all-features --no-fail-fast
exit $LASTEXITCODE
