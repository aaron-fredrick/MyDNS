import { useEffect, useState } from 'react';
import { Page } from '../components/Page';
import { api } from '../api';

type SettingsData = { resolver_mode: string; resolver_priority: string; cloudflare_dns: string; router_dns: string | null; root_hints: string[] };

const defaults: SettingsData = { resolver_mode: 'forwarding', resolver_priority: 'cloudflare_first', cloudflare_dns: '', router_dns: null, root_hints: [] };

export function Settings() {
  const [data, setData] = useState<SettingsData>(defaults);

  useEffect(() => { api.settings().then(setData).catch(console.error); }, []);

  function handleSave() {
    api.saveSettings(data).then(setData).catch(e => alert(e.message));
  }

  return (
    <Page title="Settings" subtitle="Resolver and upstream configuration.">
      <div className="settings">
        <h3>Resolver Mode</h3>
        <div className="segmented">
          <button
            id="mode-forwarding"
            type="button"
            className={data.resolver_mode === 'forwarding' ? 'selected' : ''}
            onClick={() => setData({ ...data, resolver_mode: 'forwarding' })}
          >
            Forwarding
          </button>
          <button
            id="mode-recursive"
            type="button"
            className={data.resolver_mode === 'recursive' ? 'selected' : ''}
            onClick={() => setData({ ...data, resolver_mode: 'recursive' })}
          >
            Recursive
          </button>
        </div>

        {data.resolver_mode === 'forwarding' && (
          <div style={{ marginTop: '2rem' }}>
            <h3>Upstream Priority</h3>
            <div className="segmented">
              <button
                id="priority-cloudflare"
                type="button"
                className={data.resolver_priority === 'cloudflare_first' ? 'selected' : ''}
                onClick={() => setData({ ...data, resolver_priority: 'cloudflare_first' })}
              >
                Cloudflare first
              </button>
              <button
                id="priority-router"
                type="button"
                className={data.resolver_priority === 'router_first' ? 'selected' : ''}
                onClick={() => setData({ ...data, resolver_priority: 'router_first' })}
              >
                Router first
              </button>
            </div>
            <label>
              Cloudflare DNS
              <input
                id="settings-cloudflare-dns"
                value={data.cloudflare_dns}
                onChange={e => setData({ ...data, cloudflare_dns: e.target.value })}
              />
            </label>
            <label>
              Router DNS
              <input
                id="settings-router-dns"
                value={data.router_dns ?? ''}
                onChange={e => setData({ ...data, router_dns: e.target.value || null })}
              />
            </label>
          </div>
        )}

        {data.resolver_mode === 'recursive' && (
          <div style={{ marginTop: '2rem' }}>
            <h3>Root Hints</h3>
            <p className="muted" style={{ marginBottom: '1rem', fontSize: '0.9rem' }}>
              When in recursive mode, the DNS server will resolve queries iteratively starting from these root servers.
            </p>
            <div className="card" style={{ overflow: 'auto', maxHeight: '300px' }}>
              <table className="table">
                <thead>
                  <tr>
                    <th>Root Server IP</th>
                  </tr>
                </thead>
                <tbody>
                  {data.root_hints && data.root_hints.map((hint, i) => (
                    <tr key={i}>
                      <td className="mono">{hint}</td>
                    </tr>
                  ))}
                  {(!data.root_hints || data.root_hints.length === 0) && (
                    <tr><td className="muted">No root hints loaded.</td></tr>
                  )}
                </tbody>
              </table>
            </div>
          </div>
        )}

        <button id="settings-save" className="primary" type="button" onClick={handleSave} style={{ marginTop: '2rem' }}>
          Save settings
        </button>
      </div>
    </Page>
  );
}
