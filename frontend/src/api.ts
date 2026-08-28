export type LatencyStats = { avg_ms: number; p50_ms: number; p95_ms: number; p99_ms: number };
export type Stats = {
  uptime_secs: number;
  requests_per_minute: number;
  queries_total: number;
  upstream: { requests: number; successes: number; failures: number; availability_pct: number; latency: LatencyStats };
  response_time: LatencyStats;
  cache_evictions: number;
  dns_errors: number;
  query_types: Record<string, number>;
  resolution_outcomes: Record<string, number>;
  cache_hits: number;
  cache_misses: number;
  cache_hit_rate: number;
  cache_size: number;
  record_count: number;
};

export type DnsRecord = { id: number; name: string; record_type: string; value: string; ttl: number; priority?: number | null };
export type CacheEntry = { name: string; record_type: string; value: string; ttl_remaining: number };

let token: string | null = sessionStorage.getItem('mydns_token');
export const auth = { get token() { return token; }, set token(value: string | null) { token = value; value ? sessionStorage.setItem('mydns_token', value) : sessionStorage.removeItem('mydns_token'); } };

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  headers.set('Accept', 'application/json');
  if (init.body) headers.set('Content-Type', 'application/json');
  if (token) headers.set('Authorization', `Bearer ${token}`);
  const response = await fetch(path, { ...init, headers });
  if (response.status === 401) { auth.token = null; window.location.assign('/login'); throw new Error('Unauthorized'); }
  if (!response.ok) {
    const body = await response.text();
    throw new Error(body || `Request failed (${response.status})`);
  }
  return response.status === 204 ? (undefined as T) : response.json();
}

export const api = {
  login: (username: string, password: string) => request<{ token: string }>('/api/v1/auth/login', { method: 'POST', body: JSON.stringify({ username, password }) }),
  stats: () => request<Stats>('/api/v1/stats'),
  records: () => request<DnsRecord[]>('/api/v1/records'),
  createRecord: (record: Omit<DnsRecord, 'id'>) => request<DnsRecord>('/api/v1/records', { method: 'POST', body: JSON.stringify(record) }),
  updateRecord: (id: number, record: Partial<DnsRecord>) => request<DnsRecord>(`/api/v1/records/${id}`, { method: 'PUT', body: JSON.stringify(record) }),
  deleteRecord: (id: number) => request<void>(`/api/v1/records/${id}`, { method: 'DELETE' }),
  cache: () => request<CacheEntry[]>('/api/v1/cache'),
  clearCache: () => request<void>('/api/v1/cache', { method: 'DELETE' }),
  deleteCache: (name: string, type: string) => request<void>(`/api/v1/cache/${encodeURIComponent(name)}/${encodeURIComponent(type)}`, { method: 'DELETE' }),
  settings: () => request<Record<string, unknown>>('/api/v1/settings'),
  saveSettings: (settings: Record<string, unknown>) => request<Record<string, unknown>>('/api/v1/settings', { method: 'PUT', body: JSON.stringify(settings) }),
};
