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

foreach ($target in $targets) {
    Write-Host "`n==> $($target.Name) [$($target.Triple)]" -ForegroundColor Yellow

    cargo build --release --target $target.Triple
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo build failed for target $($target.Triple)."
    }

    $targetOut = Join-Path $out $target.Name
    $binOut = Join-Path $targetOut "bin"
    $webSource = Join-Path $out "web"
    $webOut = Join-Path $targetOut "web"

    New-Item -ItemType Directory -Force -Path $binOut | Out-Null
    New-Item -ItemType Directory -Force -Path $webOut | Out-Null

    $binarySource = Join-Path $root "target\$($target.Triple)\release\$($target.Binary)"
    $binaryDestination = Join-Path $binOut $target.Binary

    if (-not (Test-Path $binarySource)) {
        throw "Expected release binary was not produced: $binarySource"
    }

    Copy-Item -Force $binarySource $binaryDestination

    if (Test-Path $webSource) {
        Copy-Item -Path (Join-Path $webSource "*") -Destination $webOut -Recurse -Force
    }
}

Write-Host "`nRelease builds completed." -ForegroundColor Green
Write-Host "Artifacts: $out"