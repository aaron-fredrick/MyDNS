import { lazy, Suspense } from 'react';
import { Navigate, Route, Routes } from 'react-router';

import { ProtectedRoute } from './ProtectedRoute';

const Login = lazy(() => import('../pages/Login').then(m => ({ default: m.Login })));
const Dashboard = lazy(() => import('../pages/Dashboard').then(m => ({ default: m.Dashboard })));
const Zones = lazy(() => import('../pages/Zones').then(m => ({ default: m.Zones })));
const Records = lazy(() => import('../pages/Records').then(m => ({ default: m.Records })));
const Cache = lazy(() => import('../pages/Cache').then(m => ({ default: m.Cache })));
const Logs = lazy(() => import('../pages/Logs').then(m => ({ default: m.Logs })));
const Settings = lazy(() => import('../pages/Settings').then(m => ({ default: m.Settings })));

function RouteLoading() {
  return <div className="route-loading" role="status" aria-live="polite">Loading…</div>;
}

export function App() {
  return (
    <Suspense fallback={<RouteLoading />}>
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route path="/" element={<ProtectedRoute><Dashboard /></ProtectedRoute>} />
        <Route path="/zones" element={<ProtectedRoute><Zones /></ProtectedRoute>} />
        <Route path="/records" element={<ProtectedRoute><Records /></ProtectedRoute>} />
        <Route path="/cache" element={<ProtectedRoute><Cache /></ProtectedRoute>} />
        <Route path="/logs" element={<ProtectedRoute><Logs /></ProtectedRoute>} />
        <Route path="/settings" element={<ProtectedRoute><Settings /></ProtectedRoute>} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Suspense>
  );
}
