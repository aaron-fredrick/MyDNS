#!/usr/bin/env python3
"""Minimal DNS smoke test using only the Python standard library."""

from __future__ import annotations

import argparse
import socket
import struct
import sys
import time


def encode_name(name: str) -> bytes:
    labels = name.rstrip(".").split(".")
    return b"".join(bytes([len(label)]) + label.encode("ascii") for label in labels) + b"\x00"


def query(
    host: str, port: int, name: str, qtype: int, tcp: bool
) -> tuple[int, int]:
    txid = int(time.time_ns() & 0xFFFF)
    packet = struct.pack("!HHHHHH", txid, 0x0100, 1, 0, 0, 0)
    packet += encode_name(name) + struct.pack("!HH", qtype, 1)

    if tcp:
        with socket.create_connection((host, port), timeout=2.0) as sock:
            sock.sendall(struct.pack("!H", len(packet)) + packet)
            length = struct.unpack("!H", sock.recv(2))[0]
            response = b""
            while len(response) < length:
                chunk = sock.recv(length - len(response))
                if not chunk:
                    raise OSError("DNS TCP connection closed before full response")
                response += chunk
    else:
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
            sock.settimeout(2.0)
            sock.sendto(packet, (host, port))
            response, _ = sock.recvfrom(4096)

    if len(response) < 12:
        raise AssertionError("DNS response is shorter than the header")

    response_id, flags, _, answers, _, _ = struct.unpack("!HHHHHH", response[:12])
    if response_id != txid:
        raise AssertionError("DNS transaction ID mismatch")

    return flags & 0x000F, answers


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=5353)
    parser.add_argument("--name", default="home.arpa.")
    parser.add_argument(
        "--tcp",
        action="store_true",
        help="Use DNS over TCP instead of UDP",
    )
    args = parser.parse_args()

    rcode, answers = query(args.host, args.port, args.name, 6, args.tcp)
    if rcode != 0:
        raise AssertionError(f"expected NOERROR (0), got rcode={rcode}")
    if answers < 1:
        raise AssertionError("expected at least one DNS answer")

    transport = "TCP" if args.tcp else "UDP"
    print(
        f"MyDNS DNS smoke: PASS ({args.name} SOA answers={answers}, "
        f"transport={transport})"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError) as exc:
        print(f"MyDNS DNS smoke: FAIL: {exc}", file=sys.stderr)
        raise SystemExit(1)
