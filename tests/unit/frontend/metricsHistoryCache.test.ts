import assert from 'node:assert/strict';
import test from 'node:test';

import { mergeHistorySamples } from '../../../src/frontend/src/utils/metricsHistoryCache.ts';

function sample(timestamp: string, requests: number) {
  return {
    timestamp,
    requests_per_minute: requests,
    response_time: { avg_ms: requests, p50_ms: requests, p95_ms: requests, p99_ms: requests },
  };
}

test('merges server history without duplicate timestamps', () => {
  const current = [
    sample('2026-08-28T10:00:00.000Z', 10),
    sample('2026-08-28T10:01:00.000Z', 11),
  ];
  const incoming = [
    sample('2026-08-28T10:01:00.000Z', 99),
    sample('2026-08-28T10:02:00.000Z', 12),
  ];

  const merged = mergeHistorySamples(current, incoming, Date.parse('2026-08-28T10:03:00.000Z'));

  assert.deepEqual(
    merged.map(item => [item.timestamp, item.requests_per_minute]),
    [
      ['2026-08-28T10:00:00.000Z', 10],
      ['2026-08-28T10:01:00.000Z', 99],
      ['2026-08-28T10:02:00.000Z', 12],
    ],
  );
});

test('prunes cached history outside the 24 hour retention window', () => {
  const now = Date.parse('2026-08-29T10:00:00.000Z');
  const merged = mergeHistorySamples(
    [
      sample('2026-08-28T09:59:00.000Z', 1),
      sample('2026-08-28T10:00:00.000Z', 2),
      sample('2026-08-29T09:59:00.000Z', 3),
    ],
    [],
    now,
  );

  assert.deepEqual(
    merged.map(item => item.timestamp),
    ['2026-08-28T10:00:00.000Z', '2026-08-29T09:59:00.000Z'],
  );
});
