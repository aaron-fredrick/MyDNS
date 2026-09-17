import assert from 'node:assert/strict';
import test from 'node:test';

import { formatUptime } from '../../../src/frontend/src/utils/formatUptime.ts';

test('formatUptime formats short uptimes as hours and minutes', () => {
  assert.equal(formatUptime(0), '0h 0m');
  assert.equal(formatUptime(65 * 60), '1h 5m');
  assert.equal(formatUptime(23 * 60 * 60 + 59 * 60), '23h 59m');
});

test('formatUptime switches to day precision for multi-day uptime', () => {
  assert.equal(formatUptime(86_400), '1d 0h');
  assert.equal(formatUptime(2 * 86_400 + 5 * 3_600 + 59 * 60), '2d 5h');
});
