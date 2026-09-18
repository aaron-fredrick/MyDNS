import { useEffect, useState } from 'react';

import { api, type HistorySample, type Stats } from '../api';
import { loadHistoryCache, mergeHistorySamples, saveHistoryCache } from '../utils/metricsHistoryCache';

const HISTORY_WINDOW_MS = 60 * 60 * 1000;
const POLL_INTERVAL_MS = 15_000;

export type DashboardSample = {
  timestamp: string;
  time: string;
  requests: number;
  avg: number;
  p95: number;
  p99: number;
};

function toDashboardSample(sample: HistorySample): DashboardSample {
  return {
    timestamp: sample.timestamp,
    time: new Date(sample.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
    requests: sample.requests_per_minute,
    avg: sample.response_time.avg_ms,
    p95: sample.response_time.p95_ms,
    p99: sample.response_time.p99_ms,
  };
}

function newestTimestamp(samples: HistorySample[], fallback: Date): Date {
  const newest = samples.at(-1)?.timestamp;
  if (!newest) {
    return new Date(fallback.getTime() - HISTORY_WINDOW_MS);
  }

  const timestamp = new Date(newest);
  return Number.isNaN(timestamp.getTime()) || timestamp > fallback
    ? new Date(fallback.getTime() - HISTORY_WINDOW_MS)
    : timestamp;
}

export function useDashboardStats() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [history, setHistory] = useState<DashboardSample[]>([]);
  const [error, setError] = useState('');

  useEffect(() => {
    let alive = true;
    let cachedSamples: HistorySample[] = [];
    let historySyncInFlight = false;

    const updateStats = async () => {
      try {
        const next = await api.stats();
        if (!alive) return;
        setStats(next);
        setError('');
      } catch (e) {
        if (alive) setError(e instanceof Error ? e.message : String(e));
      }
    };

    const syncHistory = async (initial = false) => {
      if (historySyncInFlight) return;
      historySyncInFlight = true;

      try {
        if (initial) {
          cachedSamples = await loadHistoryCache();
          if (!alive) return;
          setHistory(cachedSamples.map(toDashboardSample));
        }

        const now = new Date();
        const from = newestTimestamp(cachedSamples, now);
        const response = await api.statsHistory(from, now);

        if (!alive) return;

        cachedSamples = mergeHistorySamples(cachedSamples, response.samples);
        setHistory(cachedSamples.map(toDashboardSample));
        void saveHistoryCache(cachedSamples);
        setError('');
      } catch (e) {
        if (alive) setError(e instanceof Error ? e.message : String(e));
      } finally {
        historySyncInFlight = false;
      }
    };

    void updateStats();
    void syncHistory(true);

    const id = window.setInterval(() => {
      void updateStats();
      void syncHistory();
    }, POLL_INTERVAL_MS);

    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  return { stats, history, error };
}
