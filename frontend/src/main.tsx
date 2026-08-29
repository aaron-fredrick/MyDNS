import React from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter, Navigate, Route, Routes } from 'react-router';

import { auth } from './api';
import { Shell } from './components/Shell';

import { Login } from './pages/Login';
import { Dashboard } from './pages/Dashboard';
import { Records } from './pages/Records';
import { Cache } from './pages/Cache';
import { Logs } from './pages/Logs';
import { Settings } from './pages/Settings';
import { Zones } from './pages/Zones';

import './styles/main.css';
import './styles/components.css';
import './styles/pages.css';

function Protected({ children }: { children: React.ReactNode }) {
  if (!auth.token) return <Navigate to="/login" replace />;
  return <Shell>{children}</Shell>;
}

function App() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/" element={<Protected><Dashboard /></Protected>} />
      <Route path="/zones" element={<Protected><Zones /></Protected>} />
      <Route path="/records" element={<Protected><Records /></Protected>} />
      <Route path="/cache" element={<Protected><Cache /></Protected>} />
      <Route path="/logs" element={<Protected><Logs /></Protected>} />
      <Route path="/settings" element={<Protected><Settings /></Protected>} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </React.StrictMode>,
);
