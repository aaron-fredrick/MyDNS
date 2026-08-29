export type LatencyStats = { avg_ms: number; p50_ms: number; p95_ms: number; p99_ms: number };
export type Stats = {
  uptime_secs: number; requests_per_minute: number; queries_total: number;
  upstream: { requests: number; successes: number; failures: number; availability_pct: number; latency: LatencyStats };
  response_time: LatencyStats; cache_evictions: number; dns_errors: number;
  query_types: Record<string, number>; resolution_outcomes: Record<string, number>;
  cache_hits: number; cache_misses: number; cache_hit_rate: number; cache_size: number; record_count: number;
};
export type DnsRecord = { id: number; name: string; record_type: string; value: string; ttl: number; priority?: number | null; is_dev?: boolean };
export type Zone = { id: number; name: string; created_at: string };
export type CacheEntry = { name: string; record_type: string; ttl_remaining: number; values: string[] };
let token: string | null = sessionStorage.getItem('mydns_token');
export const auth = { get token() { return token; }, set token(value: string | null) { token = value; value ? sessionStorage.setItem('mydns_token', value) : sessionStorage.removeItem('mydns_token'); } };
async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers); headers.set('Accept', 'application/json'); if (init.body) headers.set('Content-Type', 'application/json'); if (token) headers.set('Authorization', `Bearer ${token}`);
  const response = await fetch(path, { ...init, headers });
  if (response.status === 401) { auth.token = null; window.location.assign('/login'); throw new Error('Unauthorized'); }
  if (!response.ok) throw new Error((await response.text()) || `Request failed (${response.status})`);
  return response.status === 204 ? (undefined as T) : response.json();
}
export const api = {
  login: (username: string, password: string) => request<{ token: string }>('/api/v1/auth/login', { method: 'POST', body: JSON.stringify({ username, password }) }),
  stats: () => request<Stats>('/api/v1/stats'),
  records: async () => (await request<{ records: DnsRecord[] }>('/api/v1/records')).records,
  createRecord: async (record: Omit<DnsRecord, 'id'>) => (await request<{ record: DnsRecord }>('/api/v1/records', { method: 'POST', body: JSON.stringify(record) })).record,
  updateRecord: async (id: number, record: Partial<DnsRecord>) => (await request<{ record: DnsRecord }>(`/api/v1/records/${id}`, { method: 'PUT', body: JSON.stringify(record) })).record,
  deleteRecord: (id: number) => request<{ deleted: number }>(`/api/v1/records/${id}`, { method: 'DELETE' }),
  cache: () => request<CacheEntry[]>('/api/v1/cache'),
  clearCache: () => request<void>('/api/v1/cache', { method: 'DELETE' }),
  deleteCache: (name: string, type: string) => request<void>(`/api/v1/cache/${encodeURIComponent(name)}/${encodeURIComponent(type)}`, { method: 'DELETE' }),
  settings: () => request<{ resolver_mode: string; resolver_priority: string; cloudflare_dns: string; router_dns: string | null; root_hints: string[] }>('/api/v1/settings'),
  saveSettings: (settings: Record<string, unknown>) => request<{ resolver_mode: string; resolver_priority: string; cloudflare_dns: string; router_dns: string | null; root_hints: string[] }>('/api/v1/settings', { method: 'PUT', body: JSON.stringify(settings) }),
  zones: async () => (await request<{ zones: Zone[] }>('/api/v1/zones')).zones,
  addZone: async (name: string) => (await request<{ zone: Zone }>('/api/v1/zones', { method: 'POST', body: JSON.stringify({ name }) })).zone,
  removeZone: (name: string) => request<{ removed: string }>(`/api/v1/zones/${encodeURIComponent(name)}`, { method: 'DELETE' }),
};

