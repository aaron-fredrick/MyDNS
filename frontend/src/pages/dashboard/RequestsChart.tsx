import { CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';

import { ChartCard } from '../../components/DataDisplay';
import type { DashboardSample } from '../../hooks/useDashboardStats';

interface RequestsChartProps {
  data: DashboardSample[];
}

export function RequestsChart({ data }: RequestsChartProps) {
  return (
    <ChartCard title="Requests over time" note="Live samples since dashboard load">
      <ResponsiveContainer width="100%" height={250}>
        <LineChart data={data}>
          <CartesianGrid strokeDasharray="3 3" stroke="var(--border)" />
          <XAxis dataKey="time" tick={{ fill: 'var(--text-muted)', fontSize: 12 }} />
          <YAxis tick={{ fill: 'var(--text-muted)', fontSize: 12 }} />
          <Tooltip contentStyle={{ background: 'var(--surface)', border: '1px solid var(--border)', color: 'var(--text)' }} />
          <Line type="monotone" dataKey="requests" stroke="var(--primary)" dot={false} name="Requests/min" />
        </LineChart>
      </ResponsiveContainer>
    </ChartCard>
  );
}
