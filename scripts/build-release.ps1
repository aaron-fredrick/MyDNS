$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root "out"

$targets = @(
    @{ Name = "win-x64";    Triple = "x86_64-pc-windows-msvc"; Binary = "mydns.exe" },
    @{ Name = "win-arm64";  Triple = "aarch64-pc-windows-msvc"; Binary = "mydns.exe" },
    @{ Name = "linux-x64";  Triple = "x86_64-unknown-linux-gnu"; Binary = "mydns" },
    @{ Name = "linux-arm64"; Triple = "aarch64-unknown-linux-gnu"; Binary = "mydns" }
)

Write-Host "Building MyDNS release targets..." -ForegroundColor Cyan

# Release output is generated state. Remove it first so stale binaries/assets
# cannot accidentally be packaged as part of a later release.
if (Test-Path $out) {
    Remove-Item -Recurse -Force $out
}

# The frontend is a build-time dependency only; Node.js is not required at runtime.
Push-Location $root
try {
    if (Test-Path (Join-Path $root "package-lock.json")) {
        npm ci
    } else {
        npm install
    }

    npm run typecheck
    npm run build
} finally {
    Pop-Location
}

foreach ($target in $targets) {
    Write-Host "`n==> $($target.Name) [$($target.Triple)]" -ForegroundColor Yellow

    cargo build --release --target $target.Triple
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed for target $($target.Triple)."
    }

    $targetOut = Join-Path $out $target.Name
    $binOut = Join-Path $targetOut "bin"
    $webOut = Join-Path $targetOut "web"
    $configOut = Join-Path $targetOut "config"

    New-Item -ItemType Directory -Force -Path $binOut, $webOut, $configOut | Out-Null

    $binarySource = Join-Path $root "target\$($target.Triple)\release\$($target.Binary)"
    $binaryDestination = Join-Path $binOut $target.Binary

    if (-not (Test-Path $binarySource)) {
        throw "Expected release binary was not produced: $binarySource"
    }

    Copy-Item -Force $binarySource $binaryDestination
    Copy-Item -Recurse -Force (Join-Path $out "web\*") $webOut
    Copy-Item -Force (Join-Path $root "config.toml.example") (Join-Path $configOut "mydns.toml.example")
}

Write-Host "`nRelease builds completed." -ForegroundColor Green
Write-Host "Artifacts: $out"
Write-Host "Each target contains bin/, web/, and config/ runtime assets."