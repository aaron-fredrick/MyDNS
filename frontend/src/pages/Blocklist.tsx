import React, { useState, useEffect } from 'react';
import { api, BlocklistEntry } from '../api';

export function Blocklist() {
  const [entries, setEntries] = useState<BlocklistEntry[]>([]);
  const [newDomain, setNewDomain] = useState('');
  const [newReason, setNewReason] = useState('');
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchBlocklist();
  }, []);

  const fetchBlocklist = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await api.blocklist();
      // Ensure data is an array
      if (Array.isArray(data)) {
        setEntries(data);
      } else {
        setEntries([]);
      }
    } catch (err: any) {
      setError(err.message || 'Failed to load blocklist');
    } finally {
      setLoading(false);
    }
  };

  const handleAddEntry = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newDomain.trim()) return;

    try {
      setSubmitting(true);
      setError(null);
      const newEntry = await api.addBlocklist(newDomain.trim(), true, newReason.trim() || undefined);
      setEntries((prev) => [...prev, newEntry].sort((a, b) => a.domain.localeCompare(b.domain)));
      setNewDomain('');
      setNewReason('');
    } catch (err: any) {
      setError(err.message || 'Failed to add to blocklist');
    } finally {
      setSubmitting(false);
    }
  };

  const handleRemoveEntry = async (id: number, domain: string) => {
    if (!window.confirm(`Are you sure you want to remove '${domain}' from the blocklist?`)) {
      return;
    }

    try {
      setSubmitting(true);
      setError(null);
      await api.deleteBlocklist(id);
      setEntries((prev) => prev.filter((e) => e.id !== id));
    } catch (err: any) {
      setError(err.message || 'Failed to remove entry');
    } finally {
      setSubmitting(false);
    }
  };

  const handleToggleEnabled = async (id: number, enabled: boolean) => {
    try {
      setSubmitting(true);
      setError(null);
      const updated = await api.updateBlocklist(id, { enabled });
      setEntries((prev) => prev.map((e) => (e.id === id ? updated : e)));
    } catch (err: any) {
      setError(err.message || 'Failed to update entry');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="card">
      <div className="card-header">
        <h2>Domain Blocklist</h2>
      </div>

      <div className="card-body">
        {error && <div className="error-banner">{error}</div>}

        <div className="info-banner" style={{ marginBottom: '1.5rem', padding: '1rem', backgroundColor: 'var(--bg-card)', borderRadius: '4px', borderLeft: '4px solid var(--primary)' }}>
          <p style={{ margin: '0 0 0.5rem 0' }}><strong>About Domain Blocklist</strong></p>
          <p style={{ margin: 0, fontSize: '0.9rem', color: 'var(--fg-muted)' }}>
            Domains added here will be blocked at the DNS level. Queries for these domains and their subdomains will instantly return an NXDOMAIN response, protecting your network from ads, trackers, and malware.
          </p>
        </div>

        <form onSubmit={handleAddEntry} style={{ display: 'flex', gap: '1rem', marginBottom: '2rem' }}>
          <div className="form-group" style={{ margin: 0, flex: 2 }}>
            <input
              type="text"
              placeholder="Domain (e.g., ads.example.com)"
              value={newDomain}
              onChange={(e) => setNewDomain(e.target.value)}
              disabled={submitting || loading}
              style={{ width: '100%' }}
            />
          </div>
          <div className="form-group" style={{ margin: 0, flex: 2 }}>
            <input
              type="text"
              placeholder="Reason (optional)"
              value={newReason}
              onChange={(e) => setNewReason(e.target.value)}
              disabled={submitting || loading}
              style={{ width: '100%' }}
            />
          </div>
          <button type="submit" className="button button-primary" disabled={submitting || loading || !newDomain.trim()}>
            + Add Domain
          </button>
        </form>

        {loading ? (
          <div className="loading">Loading blocklist...</div>
        ) : entries.length === 0 ? (
          <div className="empty-state">No domains in the blocklist.</div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Domain</th>
                <th>Reason</th>
                <th>Status</th>
                <th>Created</th>
                <th style={{ width: '100px', textAlign: 'right' }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {entries.map((entry) => (
                <tr key={entry.id}>
                  <td>
                    <strong>{entry.domain}</strong>
                  </td>
                  <td className="muted">{entry.reason || '-'}</td>
                  <td>
                    <button
                      className={`button button-small ${entry.enabled ? 'button-primary' : ''}`}
                      onClick={() => handleToggleEnabled(entry.id, !entry.enabled)}
                      disabled={submitting}
                      style={{ opacity: entry.enabled ? 1 : 0.7 }}
                    >
                      {entry.enabled ? 'Enabled' : 'Disabled'}
                    </button>
                  </td>
                  <td className="muted">{new Date(entry.created_at).toLocaleString()}</td>
                  <td style={{ textAlign: 'right' }}>
                    <button
                      className="button button-danger button-small"
                      onClick={() => handleRemoveEntry(entry.id, entry.domain)}
                      disabled={submitting}
                      type="button"
                    >
                      Remove
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
