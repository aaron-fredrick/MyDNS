import { useEffect, useState } from 'react';
import { CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { Metric, ChartCard } from '../components/DataDisplay';
import { Page } from '../components/Page';
import { api, type Stats } from '../api';

type Sample = { time: string; requests: number; avg: number; p95: number; p99: number };

function formatUptime(sec: number) {
  const d = Math.floor(sec / 86400);
  const h = Math.floor(sec % 86400 / 3600);
  const m = Math.floor(sec % 3600 / 60);
  return d ? `${d}d ${h}h` : `${h}h ${m}m`;
}

export function Dashboard() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [history, setHistory] = useState<Sample[]>([]);
  const [error, setError] = useState('');

  useEffect(() => {
    let alive = true;

    const load = () =>
      api.stats()
        .then(s => {
          if (!alive) return;
          setStats(s);
          setHistory(old => [
            ...old,
            {
              time: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
              requests: s.requests_per_minute,
              avg: s.response_time.avg_ms,
              p95: s.response_time.p95_ms,
              p99: s.response_time.p99_ms,
            },
          ].slice(-60));
        })
        .catch(e => alive && setError(e.message));

    load();
    const id = window.setInterval(load, 15_000);
    return () => { alive = false; clearInterval(id); };
  }, []);

  return (
    <Page title="Dashboard" subtitle="Resolver health and current performance.">
      {error && <div className="alert danger" role="alert">{error}</div>}

      <section className="metrics-grid">
        <Metric label="Cache hit rate" value={stats ? `${stats.cache_hit_rate.toFixed(1)}%` : '—'} detail="vs last hour" />
        <Metric label="Upstream latency" value={stats ? `${stats.upstream.latency.avg_ms.toFixed(1)} ms` : '—'} detail="vs last hour" />
        <Metric label="Avg response time" value={stats ? `${stats.response_time.avg_ms.toFixed(1)} ms` : '—'} detail="vs last hour" />
        <Metric label="Requests" value={stats ? `${stats.requests_per_minute.toLocaleString()}/min` : '—'} detail="current rate" />
        <Metric label="Upstream availability" value={stats ? `${stats.upstream.availability_pct.toFixed(1)}%` : '—'} detail={`${stats?.upstream.failures ?? 0} failures`} />
      </section>

      <div className="chart-grid">
        <ChartCard title="Requests over time" note="Live samples since dashboard load">
          <ResponsiveContainer width="100%" height={250}>
            <LineChart data={history}>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
              <XAxis dataKey="time" tick={{ fill: 'var(--text-muted)', fontSize: 12 }} />
              <YAxis tick={{ fill: 'var(--text-muted)', fontSize: 12 }} />
              <Tooltip contentStyle={{ background: 'var(--surface)', border: '1px solid var(--border)', color: 'var(--text)' }} />
              <Line type="monotone" dataKey="requests" stroke="var(--primary)" dot={false} name="Requests/min" />
            </LineChart>
          </ResponsiveContainer>
        </ChartCard>
        <ChartCard title="Response time" note="Backend samples: Average, P95 and P99">
          <ResponsiveContainer width="100%" height={250}>
            <LineChart data={history}>
              <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
              <XAxis dataKey="time" tick={{ fill: 'var(--text-muted)', fontSize: 12 }} />
              <YAxis unit=" ms" tick={{ fill: 'var(--text-muted)', fontSize: 12 }} />
              <Tooltip contentStyle={{ background: 'var(--surface)', border: '1px solid var(--border)', color: 'var(--text)' }} />
              <Line type="monotone" dataKey="avg" stroke="var(--primary)" dot={false} name="Average" />
              <Line type="monotone" dataKey="p95" stroke="var(--warning)" dot={false} name="P95" />
              <Line type="monotone" dataKey="p99" stroke="var(--error)" dot={false} name="P99" />
            </LineChart>
          </ResponsiveContainer>
        </ChartCard>
      </div>

      <div className="summary-grid">
        <div className="card">
          <h3>At a glance</h3>
          <p>Uptime <b>{stats ? formatUptime(stats.uptime_secs) : '—'}</b></p>
          <p>Total queries <b>{stats?.queries_total.toLocaleString() ?? '—'}</b></p>
          <p>DNS errors <b>{stats?.dns_errors.toLocaleString() ?? '—'}</b></p>
        </div>
        <div className="card">
          <h3>Latency distribution</h3>
          <p>P50 <b>{stats?.response_time.p50_ms.toFixed(1) ?? '—'} ms</b></p>
          <p>P95 <b>{stats?.response_time.p95_ms.toFixed(1) ?? '—'} ms</b></p>
          <p>P99 <b>{stats?.response_time.p99_ms.toFixed(1) ?? '—'} ms</b></p>
        </div>
      </div>
    </Page>
  );
}
