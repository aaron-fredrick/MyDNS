import React, { useState, useEffect } from 'react';
import { api, Zone } from '../api';

export function Zones() {
  const [zones, setZones] = useState<Zone[]>([]);
  const [newZoneName, setNewZoneName] = useState('');
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchZones();
  }, []);

  const fetchZones = async () => {
    try {
      setLoading(true);
      setError(null);
      const data = await api.zones();
      setZones(data || []);
    } catch (err: any) {
      setError(err.message || 'Failed to load zones');
    } finally {
      setLoading(false);
    }
  };

  const handleAddZone = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newZoneName.trim()) return;

    try {
      setSubmitting(true);
      setError(null);
      const newZone = await api.addZone(newZoneName.trim());
      setZones((prev) => [...prev, newZone].sort((a, b) => a.name.localeCompare(b.name)));
      setNewZoneName('');
    } catch (err: any) {
      setError(err.message || 'Failed to add zone');
    } finally {
      setSubmitting(false);
    }
  };

  const handleRemoveZone = async (name: string) => {
    if (!window.confirm(`Are you sure you want to remove the zone '${name}'? DNS queries for this zone will no longer be handled authoritatively.`)) {
      return;
    }

    try {
      setSubmitting(true);
      setError(null);
      await api.removeZone(name);
      setZones((prev) => prev.filter((z) => z.name !== name));
    } catch (err: any) {
      setError(err.message || 'Failed to remove zone');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="card">
      <div className="card-header">
        <h2>Authoritative Zones</h2>
      </div>

      <div className="card-body">
        {error && <div className="error-banner">{error}</div>}

        <div className="info-banner" style={{ marginBottom: '1.5rem', padding: '1rem', backgroundColor: 'var(--bg-card)', borderRadius: '4px', borderLeft: '4px solid var(--primary)' }}>
          <p style={{ margin: '0 0 0.5rem 0' }}><strong>About Authoritative Zones</strong></p>
          <p style={{ margin: '0 0 0.5rem 0', fontSize: '0.9rem', color: 'var(--fg-muted)' }}>
            These are the domains your MyDNS server is authoritative for. Any query for a name within these zones will be resolved locally using your DNS Records, and will <em>never</em> be forwarded upstream.
          </p>
          <p style={{ margin: 0, fontSize: '0.9rem', color: 'var(--fg-muted)' }}>
            <strong>Special Root Zone (.):</strong> If you add <code>.</code> as a zone, your server becomes authoritative for the <em>entire</em> internet. All queries will resolve locally (or return NXDOMAIN if no record exists).
          </p>
        </div>

        <form onSubmit={handleAddZone} style={{ display: 'flex', gap: '1rem', marginBottom: '2rem' }}>
          <div className="form-group" style={{ margin: 0, flex: 1 }}>
            <input
              type="text"
              id="newZoneName"
              placeholder="e.g. home.local or ."
              value={newZoneName}
              onChange={(e) => setNewZoneName(e.target.value)}
              disabled={submitting || loading}
              style={{ width: '100%' }}
            />
          </div>
          <button type="submit" className="button button-primary" disabled={submitting || loading || !newZoneName.trim()}>
            + Add Zone
          </button>
        </form>

        {loading ? (
          <div className="loading">Loading zones...</div>
        ) : zones.length === 0 ? (
          <div className="empty-state">No authoritative zones configured.</div>
        ) : (
          <table className="table">
            <thead>
              <tr>
                <th>Zone Name</th>
                <th>Created</th>
                <th style={{ width: '100px', textAlign: 'right' }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {zones.map((zone) => (
                <tr key={zone.id}>
                  <td>
                    <strong>{zone.name}</strong>
                    {zone.name === '.' && <span className="badge" style={{ marginLeft: '0.5rem' }}>Root Zone</span>}
                  </td>
                  <td className="muted">{new Date(zone.created_at).toLocaleString()}</td>
                  <td style={{ textAlign: 'right' }}>
                    <button
                      className="button button-danger button-small"
                      onClick={() => handleRemoveZone(zone.name)}
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
