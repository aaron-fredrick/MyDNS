import type { ReactNode } from 'react';
import { Navigate } from 'react-router';

import { auth } from '../api';
import { Shell } from '../components/Shell';

interface ProtectedRouteProps {
  children: ReactNode;
}

export function ProtectedRoute({ children }: ProtectedRouteProps) {
  if (!auth.token) return <Navigate to="/login" replace />;
  return <Shell>{children}</Shell>;
}
