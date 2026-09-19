#!/usr/bin/env python3
"""Black-box MyDNS API E2E workflow.

Usage:
  python tests/e2e/api_e2e.py --base-url http://127.0.0.1:8080 --username admin --password changeme123
"""
import argparse, json, socket, struct, sys, urllib.error, urllib.request

def dns_query(host, port, name):
    txid = int.from_bytes(__import__("time").time_ns().to_bytes(8, "big")[-2:], "big")
    labels = name.rstrip(".").split(".")
    qname = b"".join(bytes([len(label)]) + label.encode("ascii") for label in labels) + b"\x00"
    packet = struct.pack("!HHHHHH", txid, 0x0100, 1, 0, 0, 0) + qname + struct.pack("!HH", 1, 1)
    with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
        sock.settimeout(3)
        sock.sendto(packet, (host, port))
        response, _ = sock.recvfrom(4096)
    if len(response) < 12:
        raise AssertionError("DNS response is too short")
    response_id, flags, _, answers, _, _ = struct.unpack("!HHHHHH", response[:12])
    if response_id != txid:
        raise AssertionError("DNS transaction ID mismatch")
    return flags & 0xF, answers

def request(base_url, path, method="GET", token=None, body=None):
    data = json.dumps(body).encode() if body is not None else None
    headers = {"Accept": "application/json"}
    if body is not None: headers["Content-Type"] = "application/json"
    if token: headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(base_url.rstrip("/") + path, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            raw = response.read()
            return response.status, json.loads(raw) if raw else None
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode(errors="replace")
        try: payload = json.loads(raw) if raw else None
        except json.JSONDecodeError: payload = raw
        return exc.code, payload

def require(status, expected, label):
    if status != expected: raise AssertionError(f"{label}: expected HTTP {expected}, got {status}")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--username", default="admin")
    parser.add_argument("--password", required=True)
    parser.add_argument("--dns-port", type=int, default=5353)
    args = parser.parse_args()

    status, _ = request(args.base_url, "/api/v1/stats")
    require(status, 200, "stats")

    status, login = request(args.base_url, "/api/v1/auth/login", "POST", body={"username": args.username, "password": args.password})
    require(status, 200, "login")
    token = login.get("token") if isinstance(login, dict) else None
    if not token: raise AssertionError("login did not return a JWT token")

    record = {"name":"e2e-test.home.arpa","record_type":"A","value":"192.0.2.44","ttl":120}
    status, created = request(args.base_url, "/api/v1/records", "POST", token, record)
    require(status, 200, "create record")
    record_id = created["record"]["id"]

    status, records = request(args.base_url, "/api/v1/records", token=token)
    require(status, 200, "list records")
    if not any(item["id"] == record_id for item in records["records"]): raise AssertionError("created record missing")

    status, _ = request(args.base_url, f"/api/v1/records/{record_id}", "PUT", token, {"value":"192.0.2.45","ttl":300})
    require(status, 200, "update record")

    rcode, answers = dns_query("127.0.0.1", args.dns_port, "e2e-test.home.arpa.")
    if rcode != 0 or answers < 1:
        raise AssertionError(f"DNS E2E query failed: rcode={rcode}, answers={answers}")

    status, _ = request(args.base_url, f"/api/v1/records/{record_id}", "DELETE", token)
    require(status, 200, "delete record")

    status, _ = request(args.base_url, "/api/v1/zones", "POST", token, {"name":"e2e.home.arpa"})
    require(status, 200, "create zone")
    status, zones = request(args.base_url, "/api/v1/zones", token=token)
    require(status, 200, "list zones")
    if not any(zone["name"] == "e2e.home.arpa" for zone in zones["zones"]): raise AssertionError("created zone missing")
    status, _ = request(args.base_url, "/api/v1/zones/e2e.home.arpa", "DELETE", token)
    require(status, 200, "delete zone")

    status, block = request(args.base_url, "/api/v1/blocklist", "POST", token, {"domain":"blocked.e2e.test","enabled":True,"reason":"e2e"})
    require(status, 200, "create blocklist entry")
    block_id = block["id"]
    status, _ = request(args.base_url, f"/api/v1/blocklist/{block_id}", "PUT", token, {"enabled":False})
    require(status, 200, "disable blocklist entry")
    status, _ = request(args.base_url, f"/api/v1/blocklist/{block_id}", "DELETE", token)
    require(status, 200, "delete blocklist entry")

    status, _ = request(args.base_url, "/api/v1/settings", token=token)
    require(status, 200, "settings")
    status, _ = request(args.base_url, "/api/v1/stats/history", token=token)
    require(status, 200, "stats history")
    print("MyDNS API E2E: PASS")

if __name__ == "__main__":
    try: main()
    except (AssertionError, KeyError, urllib.error.URLError) as exc:
        print(f"MyDNS API E2E: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
