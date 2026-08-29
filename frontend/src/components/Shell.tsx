import React, { useCallback } from 'react';
import { NavLink, useNavigate } from 'react-router';
import { Logo } from './Logo';
import { auth } from '../api';

const NAV_ITEMS = [
  { to: '/', label: 'Dashboard', glyph: '▦' },
  { to: '/zones', label: 'Zones', glyph: '◉' },
  { to: '/records', label: 'DNS Records', glyph: '≡' },
  { to: '/cache', label: 'DNS Cache', glyph: '◫' },
  { to: '/logs', label: 'Live Logs', glyph: '⌁' },
  { to: '/settings', label: 'Settings', glyph: '⚙' },
] as const;

function ThemeToggle() {
  const toggle = useCallback(() => {
    const html = document.documentElement;
    html.setAttribute('data-theme', html.getAttribute('data-theme') === 'light' ? 'dark' : 'light');
  }, []);
  return (
    <button className="button" onClick={toggle} type="button" id="theme-toggle">
      Toggle theme
    </button>
  );
}

interface ShellProps {
  children: React.ReactNode;
}

export function Shell({ children }: ShellProps) {
  const navigate = useNavigate();

  const handleLogout = useCallback(() => {
    auth.token = null;
    navigate('/login');
  }, [navigate]);

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <Logo />
        <nav className="nav" aria-label="Primary navigation">
          {NAV_ITEMS.map(({ to, label, glyph }) => (
            <NavLink
              key={to}
              to={to}
              end={to === '/'}
              className={({ isActive }) => (isActive ? 'active' : undefined)}
            >
              <span className="nav-glyph" aria-hidden="true">{glyph}</span>
              {label}
            </NavLink>
          ))}
        </nav>
        <button className="logout" onClick={handleLogout} type="button">
          ↪ Logout
        </button>
      </aside>
      <main className="main">
        <header className="topbar">
          <span className="muted">MyDNS Dashboard</span>
          <ThemeToggle />
        </header>
        {children}
      </main>
    </div>
  );
}
