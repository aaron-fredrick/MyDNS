import { Metric } from '../components/DataDisplay';
import { Page } from '../components/Page';
import { useDashboardStats } from '../hooks/useDashboardStats';
import { formatUptime } from '../utils/formatUptime';
import { RequestsChart } from './dashboard/RequestsChart';
import { ResponseTimeChart } from './dashboard/ResponseTimeChart';

export function Dashboard() {
  const { stats, history, error } = useDashboardStats();

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
        <RequestsChart data={history} />
        <ResponseTimeChart data={history} />
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
