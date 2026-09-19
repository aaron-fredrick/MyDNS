# MyDNS HTTP smoke test. Requires a running MyDNS instance and PowerShell 7+.
# Example: .\tests\smoke\api-smoke.ps1 -BaseUrl http://127.0.0.1:8080

param([string]$BaseUrl = "http://127.0.0.1:8080")
$ErrorActionPreference = "Stop"

function Assert-Status {
    param([int]$Actual, [int]$Expected, [string]$Name)
    if ($Actual -ne $Expected) { throw "$Name failed: expected HTTP $Expected, got HTTP $Actual" }
}

Write-Host "MyDNS API smoke test: $BaseUrl" -ForegroundColor Cyan
$stats = Invoke-WebRequest "$BaseUrl/api/v1/stats" -UseBasicParsing
Assert-Status $stats.StatusCode 200 "GET /api/v1/stats"

if ([string]::IsNullOrWhiteSpace($env:MYDNS_ADMIN_PASSWORD)) {
    throw "Set MYDNS_ADMIN_PASSWORD before running the smoke test."
}

$loginBody = @{ username = "admin"; password = $env:MYDNS_ADMIN_PASSWORD } | ConvertTo-Json
$login = Invoke-WebRequest "$BaseUrl/api/v1/auth/login" -Method Post -ContentType "application/json" -Body $loginBody -UseBasicParsing
Assert-Status $login.StatusCode 200 "POST /api/v1/auth/login"

$token = ($login.Content | ConvertFrom-Json).token
if ([string]::IsNullOrWhiteSpace($token)) { throw "Login response did not contain a token." }
$headers = @{ Authorization = "Bearer $token" }

$records = Invoke-WebRequest "$BaseUrl/api/v1/records" -Headers $headers -UseBasicParsing
Assert-Status $records.StatusCode 200 "GET /api/v1/records"
$history = Invoke-WebRequest "$BaseUrl/api/v1/stats/history" -Headers $headers -UseBasicParsing
Assert-Status $history.StatusCode 200 "GET /api/v1/stats/history"

Write-Host "MyDNS API smoke test: PASS" -ForegroundColor Green
