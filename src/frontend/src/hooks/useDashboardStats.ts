import { useEffect, useState } from 'react';

import { api, type Stats } from '../api';

export type DashboardSample = {
  time: string;
  requests: number;
  avg: number;
  p95: number;
  p99: number;
};

export function useDashboardStats() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [history, setHistory] = useState<DashboardSample[]>([]);
  const [error, setError] = useState('');

  useEffect(() => {
    let alive = true;

    const load = () =>
      api.stats()
        .then(next => {
          if (!alive) return;
          setStats(next);
          setError('');
          setHistory(old => [
            ...old,
            {
              time: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
              requests: next.requests_per_minute,
              avg: next.response_time.avg_ms,
              p95: next.response_time.p95_ms,
              p99: next.response_time.p99_ms,
            },
          ].slice(-60));
        })
        .catch(e => alive && setError(e instanceof Error ? e.message : String(e)));

    load();
    const id = window.setInterval(load, 15_000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  return { stats, history, error };
}
