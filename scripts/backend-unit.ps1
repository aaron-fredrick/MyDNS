$ErrorActionPreference = "Stop"

npm --prefix src/frontend ci
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

npm --prefix src/frontend run build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

cargo test --lib --all-features --no-fail-fast
exit $LASTEXITCODE
