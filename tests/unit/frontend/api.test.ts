import assert from 'node:assert/strict';
import test from 'node:test';

class MemoryStorage {
  private values = new Map<string, string>();
  getItem(key: string) { return this.values.get(key) ?? null; }
  setItem(key: string, value: string) { this.values.set(key, value); }
  removeItem(key: string) { this.values.delete(key); }
  clear() { this.values.clear(); }
}

const storage = new MemoryStorage();
const redirects: string[] = [];
(globalThis as any).sessionStorage = storage;
(globalThis as any).window = { location: { assign: (path: string) => redirects.push(path) } };

const { api, auth } = await import('../../../src/frontend/src/api.ts');

test('API client attaches bearer tokens to requests', async () => {
  auth.token = 'test-token';
  let seen: RequestInit | undefined;
  (globalThis as any).fetch = async (_input: string, init?: RequestInit) => {
    seen = init;
    return new Response(JSON.stringify({ uptime_secs: 1 }), { status: 200, headers: { 'Content-Type': 'application/json' } });
  };

  await api.stats();
  assert.equal(new Headers(seen?.headers).get('Authorization'), 'Bearer test-token');
  assert.equal(new Headers(seen?.headers).get('Accept'), 'application/json');
});

test('API client sends JSON bodies for mutations', async () => {
  auth.token = null;
  let seen: RequestInit | undefined;
  (globalThis as any).fetch = async (_input: string, init?: RequestInit) => {
    seen = init;
    return new Response(JSON.stringify({ token: 'new-token' }), { status: 200 });
  };

  const result = await api.login('admin', 'password');
  assert.deepEqual(result, { token: 'new-token' });
  assert.equal(seen?.method, 'POST');
  assert.equal(new Headers(seen?.headers).get('Content-Type'), 'application/json');
  assert.deepEqual(JSON.parse(String(seen?.body)), { username: 'admin', password: 'password' });
});

test('API client clears auth and redirects after 401', async () => {
  auth.token = 'expired-token';
  redirects.length = 0;
  (globalThis as any).fetch = async () => new Response('Unauthorized', { status: 401 });

  await assert.rejects(() => api.stats(), /Unauthorized/);
  assert.equal(auth.token, null);
  assert.deepEqual(redirects, ['/login']);
});
