$ErrorActionPreference = "Stop"
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo clippy --all-targets --all-features -- -D warnings
exit $LASTEXITCODE
