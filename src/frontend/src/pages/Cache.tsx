import { useEffect, useState } from 'react';
import { Page } from '../components/Page';
import { api, type CacheEntry } from '../api';

export function Cache() {
  const [entries, setEntries] = useState<CacheEntry[]>([]);

  async function load() {
    setEntries(await api.cache());
  }

  useEffect(() => { load().catch(console.error); }, []);

  async function handleClearAll() {
    await api.clearCache();
    await load();
  }

  async function handleRemove(name: string, recordType: string) {
    await api.deleteCache(name, recordType);
    await load();
  }

  return (
    <Page title="DNS Cache" subtitle="Inspect and clear resolver cache entries.">
      <div className="toolbar">
        <span className="muted">{entries.length} cached entries</span>
        <div className="spacer" />
        <button
          id="cache-clear-all"
          className="danger-button"
          type="button"
          onClick={() => handleClearAll().catch(e => alert(e.message))}
        >
          Clear all
        </button>
      </div>

      <div className="card" style={{ overflow: 'auto' }}>
        <table>
          <thead>
            <tr>
              <th>Name</th><th>Type</th><th>Value</th><th>TTL remaining</th><th />
            </tr>
          </thead>
          <tbody>
            {entries.length === 0
              ? <tr><td colSpan={5} className="muted">Cache is empty.</td></tr>
              : entries.map((entry, i) => (
                <tr key={`${entry.name}-${entry.record_type}-${i}`}>
                  <td className="mono">{entry.name}</td>
                  <td><span className="tag">{entry.record_type}</span></td>
                  <td className="mono value-cell">{entry.values.join(', ')}</td>
                  <td>{entry.ttl_remaining}s</td>
                  <td>
                    <button
                      className="danger-link"
                      type="button"
                      onClick={() => handleRemove(entry.name, entry.record_type).catch(e => alert(e.message))}
                    >
                      Remove
                    </button>
                  </td>
                </tr>
              ))
            }
          </tbody>
        </table>
      </div>
    </Page>
  );
}
