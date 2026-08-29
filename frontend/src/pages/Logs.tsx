import { useEffect, useState } from 'react';
import { Page } from '../components/Page';
import { auth } from '../api';

export function Logs() {
  const [logs, setLogs] = useState<string[]>([]);

  useEffect(() => {
    if (!auth.token) return;
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    const ws = new WebSocket(`${proto}://${location.host}/ws`, [`mydns-auth.${auth.token}`]);
    ws.onmessage = e => setLogs(old => [...old.slice(-199), String(e.data)]);
    return () => ws.close();
  }, []);

  return (
    <Page title="Live Logs" subtitle="Real-time resolver events from the Rust backend.">
      <div className="log-panel">
        {logs.length
          ? logs.map((log, i) => (
            <div className="log-line" key={i}>
              <span>{new Date().toLocaleTimeString()}</span>
              {log}
            </div>
          ))
          : <div className="empty">Waiting for resolver events…</div>
        }
      </div>
    </Page>
  );
}
