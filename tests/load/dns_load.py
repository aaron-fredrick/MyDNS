#!/usr/bin/env python3
"""Controlled DNS load generator for local MyDNS performance testing.

The workload is phase-based so a test can hold a manageable baseline rate and
then abruptly burst to a higher rate. Defaults are deliberately bounded; use
--max-rps to make the safety ceiling explicit when increasing load.
"""

from __future__ import annotations

import argparse
import json
import socket
import statistics
import struct
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Result:
    latency_ms: float
    ok: bool
    error: str | None = None


def encode_name(name: str) -> bytes:
    name = name.rstrip(".")
    return b"".join(bytes([len(label)]) + label.encode("ascii") for label in name.split(".")) + b"\x00"


def query_packet(name: str, qtype: int = 1, txid: int = 0x4242) -> bytes:
    flags = 0x0100  # RD
    header = struct.pack("!HHHHHH", txid, flags, 1, 0, 0, 0)
    question = encode_name(name) + struct.pack("!HH", qtype, 1)
    return header + question


def one_query(host: str, port: int, name: str, timeout: float) -> Result:
    started = time.perf_counter()
    try:
        with socket.socket(socket.AF_INET, socket.SOCK_DGRAM) as sock:
            sock.settimeout(timeout)
            packet = query_packet(name, txid=int(time.time_ns() & 0xFFFF))
            sock.sendto(packet, (host, port))
            response, _ = sock.recvfrom(4096)
            if len(response) < 12:
                raise RuntimeError("short DNS response")
            txid, flags = struct.unpack("!HH", response[:4])
            if txid != struct.unpack("!H", packet[:2])[0]:
                raise RuntimeError("transaction id mismatch")
            rcode = flags & 0x000F
            if rcode not in (0, 3):
                raise RuntimeError(f"DNS rcode={rcode}")
        return Result((time.perf_counter() - started) * 1000, True)
    except Exception as exc:  # noqa: BLE001 - load tool reports request failures
        return Result((time.perf_counter() - started) * 1000, False, str(exc))


def run_phase(host: str, port: int, queries: list[str], rps: float, duration: float, workers: int, timeout: float, max_rps: float) -> list[Result]:
    if rps < 0 or rps > max_rps:
        raise ValueError(f"phase rps={rps} exceeds safety ceiling {max_rps}")
    deadline = time.monotonic() + duration
    results: list[Result] = []
    submitted = 0
    accumulator = 0.0
    tick = 0.1
    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = []
        while time.monotonic() < deadline:
            accumulator += rps * tick
            count = int(accumulator)
            accumulator -= count
            for _ in range(count):
                name = queries[submitted % len(queries)]
                futures.append(pool.submit(one_query, host, port, name, timeout))
                submitted += 1
            time.sleep(tick)
        results.extend(f.result() for f in futures)
    return results


def summarize(name: str, results: list[Result], elapsed: float) -> None:
    latencies = sorted(r.latency_ms for r in results)
    failures = sum(not r.ok for r in results)

    def pct(value: float) -> float:
        if not latencies:
            return 0.0
        index = min(len(latencies) - 1, int(len(latencies) * value / 100))
        return latencies[index]

    rate = len(results) / elapsed if elapsed else 0.0
    print(
        f"{name}: requests={len(results)} rate={rate:.1f}/s failures={failures} "
        f"p50={pct(50):.2f}ms p95={pct(95):.2f}ms p99={pct(99):.2f}ms "
        f"avg={statistics.mean(latencies) if latencies else 0:.2f}ms"
    )
    if failures:
        examples = [r.error for r in results if r.error][:5]
        print(f"  failure examples: {examples}")


def load_scenario(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scenario", type=Path, help="JSON workload scenario")
    parser.add_argument("--host", default=None)
    parser.add_argument("--port", type=int, default=None)
    parser.add_argument("--max-rps", type=float, default=None)
    args = parser.parse_args()

    scenario = load_scenario(args.scenario) if args.scenario else {
        "host": "127.0.0.1",
        "port": 5353,
        "max_rps": 250,
        "workers": 32,
        "timeout_seconds": 1.0,
        "queries": ["example.com.", "example.net.", "example.org."],
        "phases": [
            {"name": "warmup", "rps": 10, "duration_seconds": 5},
            {"name": "sustain", "rps": 25, "duration_seconds": 15},
            {"name": "burst", "rps": 150, "duration_seconds": 5},
            {"name": "recovery", "rps": 25, "duration_seconds": 10},
        ],
    }

    host = args.host or scenario["host"]
    port = args.port or scenario["port"]
    max_rps = args.max_rps or scenario.get("max_rps", 250)
    queries = scenario["queries"]
    workers = scenario.get("workers", 32)
    timeout = scenario.get("timeout_seconds", 1.0)

    if not queries:
        raise SystemExit("scenario must contain at least one query")

    print(f"DNS target: {host}:{port}; safety ceiling: {max_rps}/s")
    for phase in scenario["phases"]:
        started = time.perf_counter()
        results = run_phase(host, port, queries, phase["rps"], phase["duration_seconds"], workers, timeout, max_rps)
        summarize(phase["name"], results, time.perf_counter() - started)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
