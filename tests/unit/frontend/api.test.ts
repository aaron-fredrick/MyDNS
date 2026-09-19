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


test('API client encodes cache deletion path segments and handles 204 responses', async () => {
  auth.token = 'test-token';
  let seenUrl = '';
  (globalThis as any).fetch = async (input: string) => {
    seenUrl = input;
    return new Response(null, { status: 204 });
  };

  await api.deleteCache('host.example.com.', 'A/AAAA');
  assert.equal(
    seenUrl,
    '/api/v1/cache/host.example.com.%2F/A%2FA',
  );
});

test('API client surfaces non-JSON HTTP errors', async () => {
  auth.token = null;
  (globalThis as any).fetch = async () => new Response('backend exploded', { status: 500 });

  await assert.rejects(() => api.stats(), /backend exploded/);
});


test('API client covers the remaining CRUD and settings helpers', async () => {
  auth.token = 'test-token';
  const calls: Array<{ input: string; init?: RequestInit }> = [];
  (globalThis as any).fetch = async (input: string, init?: RequestInit) => {
    calls.push({ input, init });
    const body = JSON.parse(String(init?.body || '{}'));
    if (input === '/api/v1/records') {
      return new Response(JSON.stringify(init?.method === 'POST'
        ? { record: { id: 7, ...body } }
        : { records: [] }), { status: 200 });
    }
    if (input === '/api/v1/records/7') {
      return new Response(JSON.stringify({ record: { id: 7, ...body } }), { status: 200 });
    }
    if (input === '/api/v1/zones') {
      return new Response(JSON.stringify({ zones: [{ id: 1, name: 'home.arpa', created_at: '2026-01-01T00:00:00Z' }] }), { status: 200 });
    }
    if (input === '/api/v1/zones/new%20zone') {
      return new Response(JSON.stringify({ zone: { id: 2, name: 'new zone', created_at: '2026-01-01T00:00:00Z' } }), { status: 200 });
    }
    if (input.startsWith('/api/v1/zones/')) {
      return new Response(JSON.stringify({ removed: 'home.arpa' }), { status: 200 });
    }
    if (input === '/api/v1/cache') {
      return new Response(JSON.stringify([]), { status: 200 });
    }
    if (input.startsWith('/api/v1/cache/')) {
      return new Response(null, { status: 204 });
    }
    if (input === '/api/v1/settings') {
      return new Response(JSON.stringify({
        resolver_mode: body.resolver_mode || 'forwarding',
        resolver_priority: body.resolver_priority || 'cloudflare_first',
        cloudflare_dns: body.cloudflare_dns || '1.1.1.1:53',
        router_dns: body.router_dns ?? null,
        root_hints: [],
      }), { status: 200 });
    }
    if (input === '/api/v1/blocklist') {
      return new Response(JSON.stringify([{ id: 1, domain: 'blocked.test', enabled: true, source: 'manual', created_at: '', updated_at: '' }]), { status: 200 });
    }
    if (input.startsWith('/api/v1/blocklist/')) {
      return new Response(init?.method === 'DELETE' ? null : JSON.stringify({ id: 1, domain: 'blocked.test', enabled: false, source: 'manual', created_at: '', updated_at: '' }), { status: init?.method === 'DELETE' ? 204 : 200 });
    }
    throw new Error(`unexpected request: ${input}`);
  };

  assert.deepEqual(await api.records(), []);
  assert.equal((await api.createRecord({ name: 'host.home.arpa', record_type: 'A', value: '192.0.2.1', ttl: 60 })).id, 7);
  assert.equal((await api.updateRecord(7, { value: '192.0.2.2' })).value, '192.0.2.2');
  await api.deleteRecord(7);

  assert.equal((await api.zones())[0].name, 'home.arpa');
  assert.equal((await api.addZone('new zone')).name, 'new zone');
  await api.removeZone('home.arpa');

  assert.deepEqual(await api.cache(), []);
  await api.clearCache();
  await api.deleteCache('host.home.arpa', 'A');

  assert.equal((await api.settings()).resolver_mode, 'forwarding');
  assert.equal((await api.saveSettings({ resolver_mode: 'recursive' })).resolver_mode, 'recursive');

  assert.equal((await api.blocklist())[0].domain, 'blocked.test');
  assert.equal((await api.addBlocklist('blocked.test')).domain, 'blocked.test');
  assert.equal((await api.updateBlocklist(1, { enabled: false })).enabled, false);
  await api.deleteBlocklist(1);

  assert.equal(calls.length, 18);
});
