$ErrorActionPreference = "Stop"
cargo clippy --all-targets --all-features -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo check --all-targets --all-features
exit $LASTEXITCODE
