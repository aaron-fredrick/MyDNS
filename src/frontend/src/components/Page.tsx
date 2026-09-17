import React from 'react';

interface PageProps {
  title: string;
  subtitle?: string;
  badge?: React.ReactNode;
  children: React.ReactNode;
}

export function Page({ title, subtitle, badge, children }: PageProps) {
  return (
    <div className="content">
      <header className="page-header">
        <div>
          <h1>{title}</h1>
          {subtitle && <p>{subtitle}</p>}
        </div>
        {badge ?? (
          <span className="status">
            <i aria-hidden="true" />
            Resolver online
          </span>
        )}
      </header>
      {children}
    </div>
  );
}
