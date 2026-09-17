#!/usr/bin/env python3
"""Controlled HTTP API load generator with the same hold/burst/recovery model."""

from __future__ import annotations

import argparse
import json
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Result:
    latency_ms: float
    ok: bool
    status: int | None
    error: str | None = None


def request(url: str, timeout: float, token: str | None) -> Result:
    started = time.perf_counter()
    headers = {"Accept": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    try:
        req = urllib.request.Request(url, headers=headers, method="GET")
        with urllib.request.urlopen(req, timeout=timeout) as response:
            response.read()
            return Result((time.perf_counter() - started) * 1000, 200 <= response.status < 400, response.status)
    except urllib.error.HTTPError as exc:
        return Result((time.perf_counter() - started) * 1000, False, exc.code, str(exc))
    except Exception as exc:  # noqa: BLE001 - load tool reports request failures
        return Result((time.perf_counter() - started) * 1000, False, None, str(exc))


def run_phase(urls: list[str], rps: float, duration: float, workers: int, timeout: float, max_rps: float, token: str | None) -> list[Result]:
    if rps > max_rps:
        raise ValueError(f"phase rps={rps} exceeds safety ceiling {max_rps}")
    deadline = time.monotonic() + duration
    futures = []
    submitted = 0
    accumulator = 0.0
    with ThreadPoolExecutor(max_workers=workers) as pool:
        while time.monotonic() < deadline:
            accumulator += rps * 0.1
            count = int(accumulator)
            accumulator -= count
            for _ in range(count):
                futures.append(pool.submit(request, urls[submitted % len(urls)], timeout, token))
                submitted += 1
            time.sleep(0.1)
    return [future.result() for future in futures]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scenario", type=Path, required=True)
    parser.add_argument("--token", help="Optional bearer token for authenticated endpoints")
    args = parser.parse_args()
    scenario = json.loads(args.scenario.read_text(encoding="utf-8"))
    urls = scenario["urls"]
    max_rps = scenario.get("max_rps", 100)
    workers = scenario.get("workers", 16)
    timeout = scenario.get("timeout_seconds", 2.0)
    for phase in scenario["phases"]:
        started = time.perf_counter()
        results = run_phase(urls, phase["rps"], phase["duration_seconds"], workers, timeout, max_rps, args.token)
        elapsed = time.perf_counter() - started
        latencies = sorted(r.latency_ms for r in results)
        failures = sum(not r.ok for r in results)
        pct = lambda p: latencies[min(len(latencies) - 1, int(len(latencies) * p / 100))] if latencies else 0.0
        print(f"{phase['name']}: requests={len(results)} rate={len(results)/elapsed if elapsed else 0:.1f}/s failures={failures} p50={pct(50):.2f}ms p95={pct(95):.2f}ms p99={pct(99):.2f}ms")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
