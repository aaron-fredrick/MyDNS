# MyDNS DNS smoke test
# Run from the repository root:
#   .\scripts\dns-smoke-test.ps1
#
# The MyDNS server must already be running on 127.0.0.1:53.

$Server = "127.0.0.1"

function Run-Test {
    param(
        [string]$Name,
        [string[]]$Arguments
    )

    Write-Host ""
    Write-Host "============================================================" -ForegroundColor Cyan
    Write-Host $Name -ForegroundColor Cyan
    Write-Host "============================================================" -ForegroundColor Cyan
    & nslookup @Arguments $Server
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "nslookup exited with code $LASTEXITCODE"
    }
}

Write-Host "MyDNS DNS smoke test" -ForegroundColor Green
Write-Host "Server: $Server"
Write-Host "Make sure target\release\mydns.exe is running before continuing."

Run-Test "A record - example.com" @("example.com")
Run-Test "AAAA record - example.com" @("-type=AAAA", "example.com")
Run-Test "A record - google.com" @("google.com")
Run-Test "AAAA record - google.com" @("-type=AAAA", "google.com")
Run-Test "A record - cloudflare.com" @("cloudflare.com")
Run-Test "AAAA record - cloudflare.com" @("-type=AAAA", "cloudflare.com")
Run-Test "MX record - gmail.com" @("-type=MX", "gmail.com")
Run-Test "NS record - example.com" @("-type=NS", "example.com")
Run-Test "TXT record - example.com" @("-type=TXT", "example.com")
Run-Test "CNAME record - www.example.com" @("-type=CNAME", "www.example.com")
Run-Test "PTR record - 127.0.0.1" @("127.0.0.1")
Run-Test "NXDOMAIN - definitely-does-not-exist-123456789.com" @("definitely-does-not-exist-123456789.com")

Write-Host ""
Write-Host "============================================================" -ForegroundColor Yellow
Write-Host "CACHE TEST - cloudflare.com A (run twice)" -ForegroundColor Yellow
Write-Host "============================================================" -ForegroundColor Yellow
& nslookup "cloudflare.com" $Server
& nslookup "cloudflare.com" $Server

Write-Host ""
Write-Host "============================================================" -ForegroundColor Yellow
Write-Host "CACHE TEST - cloudflare.com AAAA (run twice)" -ForegroundColor Yellow
Write-Host "============================================================" -ForegroundColor Yellow
& nslookup "-type=AAAA" "cloudflare.com" $Server
& nslookup "-type=AAAA" "cloudflare.com" $Server

Write-Host ""
Write-Host "============================================================" -ForegroundColor Yellow
Write-Host "REPEAT TEST - google.com A (10 requests)" -ForegroundColor Yellow
Write-Host "============================================================" -ForegroundColor Yellow
1..10 | ForEach-Object {
    Write-Host "Request $_/10"
    & nslookup "google.com" $Server
}

Write-Host ""
Write-Host "============================================================" -ForegroundColor Green
Write-Host "DNS smoke test complete." -ForegroundColor Green
Write-Host "Review the MyDNS server console for [UPSTREAM], [CACHE HIT], and [NXDOMAIN] entries."
