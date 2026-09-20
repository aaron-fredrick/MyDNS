param(
    [string]$DnsHost = "127.0.0.1",
    [int]$Port = 5353,
    [string]$Name = "home.arpa."
)

$ErrorActionPreference = "Stop"

function Encode-DnsName([string]$Name) {
    $bytes = [System.Collections.Generic.List[byte]]::new()
    foreach ($label in $Name.TrimEnd(".").Split(".")) {
        $labelBytes = [System.Text.Encoding]::ASCII.GetBytes($label)
        if ($labelBytes.Length -gt 63) {
            throw "DNS label is too long"
        }
        $bytes.Add([byte]$labelBytes.Length)
        $bytes.AddRange($labelBytes)
    }
    $bytes.Add(0)
    return $bytes.ToArray()
}

function Read-U16([byte[]]$Bytes, [int]$Offset) {
    return ([int]$Bytes[$Offset] -shl 8) -bor [int]$Bytes[$Offset + 1]
}

$random = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$txidBytes = New-Object byte[] 2
$random.GetBytes($txidBytes)
$random.Dispose()
$txid = Read-U16 $txidBytes 0

$packet = [System.Collections.Generic.List[byte]]::new()
$packet.Add([byte](($txid -shr 8) -band 0xff))
$packet.Add([byte]($txid -band 0xff))
$packet.Add(0x01)
$packet.Add(0x00)
$packet.Add(0x00)
$packet.Add(0x01)
$packet.Add(0x00)
$packet.Add(0x00)
$packet.Add(0x00)
$packet.Add(0x00)
$packet.Add(0x00)
$packet.Add(0x00)
$packet.AddRange((Encode-DnsName $Name))
$packet.Add(0x00)
$packet.Add(0x06)
$packet.Add(0x00)
$packet.Add(0x01)

$client = [System.Net.Sockets.UdpClient]::new()
try {
    $client.Client.ReceiveTimeout = 2000
    $endpoint = [System.Net.IPEndPoint]::new([System.Net.IPAddress]::Parse($DnsHost), $Port)
    [void]$client.Send($packet.ToArray(), $packet.Count, $DnsHost, $Port)
    $response = $client.Receive([ref]$endpoint)

    if ($response.Length -lt 12) {
        throw "DNS response is shorter than the header"
    }

    $responseId = Read-U16 $response 0
    $flags = Read-U16 $response 2
    $answers = Read-U16 $response 6
    $rcode = $flags -band 0x000f

    if ($responseId -ne $txid) {
        throw "DNS transaction ID mismatch"
    }
    if ($rcode -ne 0) {
        throw "expected NOERROR (0), got rcode=$rcode"
    }
    if ($answers -lt 1) {
        throw "expected at least one DNS answer"
    }

    Write-Output ("MyDNS DNS smoke: PASS ({0} SOA answers={1})" -f $Name, $answers)
}
finally {
    $client.Dispose()
}
