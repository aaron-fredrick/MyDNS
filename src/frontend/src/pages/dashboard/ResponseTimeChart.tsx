import { CartesianGrid, Legend, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';

import { ChartCard } from '../../components/DataDisplay';
import type { DashboardSample } from '../../hooks/useDashboardStats';

interface ResponseTimeChartProps {
  data: DashboardSample[];
}

export function ResponseTimeChart({ data }: ResponseTimeChartProps) {
  return (
    <ChartCard title="Response time" note="Backend samples: Average, P95 and P99">
      <ResponsiveContainer width="100%" height={250}>
        <LineChart data={data}>
          <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
          <XAxis dataKey="time" tick={{ fill: 'var(--text-muted)', fontSize: 12 }} />
          <YAxis unit=" ms" tick={{ fill: 'var(--text-muted)', fontSize: 12 }} />
          <Tooltip contentStyle={{ background: 'var(--surface)', border: '1px solid var(--border)', color: 'var(--text)' }} />
          <Legend wrapperStyle={{ color: 'var(--text-muted)', fontSize: 12 }} />
          <Line type="monotone" dataKey="avg" stroke="var(--primary)" dot={false} name="Average" />
          <Line type="monotone" dataKey="p95" stroke="var(--warning)" dot={false} name="P95" />
          <Line type="monotone" dataKey="p99" stroke="var(--error)" dot={false} name="P99" />
        </LineChart>
      </ResponsiveContainer>
    </ChartCard>
  );
}
