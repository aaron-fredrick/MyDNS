import React from 'react';

interface MetricProps {
  label: string;
  value: string;
  detail?: string;
}

export function Metric({ label, value, detail }: MetricProps) {
  return (
    <article className="metric">
      <div className="eyebrow">{label}</div>
      <strong>{value}</strong>
      {detail && <small>{detail}</small>}
    </article>
  );
}

interface ChartCardProps {
  title: string;
  note: string;
  children: React.ReactNode;
}

export function ChartCard({ title, note, children }: ChartCardProps) {
  return (
    <div className="card chart-card">
      <div className="panel-head">
        <div>
          <h3>{title}</h3>
          <span>{note}</span>
        </div>
      </div>
      {children}
    </div>
  );
}
