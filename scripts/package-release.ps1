param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("win-x64", "win-arm64", "linux-x64", "linux-arm64")]
    [string]$Target,

    [string]$Version = "1.0.0",

    [switch]$Installer
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root "out"
$targetDir = Join-Path $out $Target
$archiveDir = Join-Path $out "packages"

if (-not (Test-Path $targetDir)) {
    throw "Release layout not found: $targetDir. Run build-release.ps1 first."
}

New-Item -ItemType Directory -Force -Path $archiveDir | Out-Null

$archive = Join-Path $archiveDir "MyDNS-v$Version-$Target.zip"
if (Test-Path $archive) {
    Remove-Item -Force $archive
}

Compress-Archive -Path (Join-Path $targetDir "*") -DestinationPath $archive
Write-Host "Portable package: $archive" -ForegroundColor Green

if ($Installer) {
    if ($Target -notin @("win-x64", "win-arm64")) {
        throw "Inno Setup installers are only supported for Windows targets."
    }

    $iss = Join-Path $root "scripts\installer\mydns-$Target.iss"
    if (-not (Test-Path $iss)) {
        throw "Installer definition not found: $iss"
    }

    $iscc = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if (-not $iscc) {
        $defaultIscc = Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"
        if (Test-Path $defaultIscc) {
            $iscc = Get-Command $defaultIscc
        }
    }

    if (-not $iscc) {
        throw "ISCC.exe was not found. Install Inno Setup 6 before building a Windows installer."
    }

    & $iscc.Source "/DMyDNSVersion=$Version" $iss
    if ($LASTEXITCODE -ne 0) {
        throw "Inno Setup failed for $Target."
    }

    Write-Host "Installer generated under $out\installers" -ForegroundColor Green
}
