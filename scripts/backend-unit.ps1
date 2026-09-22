$ErrorActionPreference = "Stop"

npm ci
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

npm run build
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if (Test-Path "out\web") {
    Remove-Item -Recurse -Force "out\web"
}
New-Item -ItemType Directory -Force "out" | Out-Null
Move-Item "src\web\dist" "out\web"

cargo test --lib --all-features --no-fail-fast
exit $LASTEXITCODE
