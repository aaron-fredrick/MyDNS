import { useEffect, useState } from 'react';
import { Page } from '../components/Page';
import { api, type DnsRecord, type Zone } from '../api';

const RECORD_TYPES = ['A', 'AAAA', 'CNAME', 'MX', 'NS', 'TXT', 'PTR', 'SOA'] as const;

const defaultForm = { name: '', record_type: 'A', value: '', ttl: 300, priority: 10, is_dev: false, zoneSuffix: '' };

export function Records() {
  const [records, setRecords] = useState<DnsRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState('');
  const [typeFilter, setTypeFilter] = useState('ALL');
  const [zones, setZones] = useState<Zone[]>([]);
  const [form, setForm] = useState(defaultForm);

  async function load() {
    setLoading(true);
    try {
      const [recs, zns] = await Promise.all([api.records(), api.zones()]);
      setRecords(recs);
      setZones(zns || []);
      
      // Select the first zone by default if available
      if (zns && zns.length > 0 && !form.zoneSuffix) {
        setForm(f => ({ ...f, zoneSuffix: zns[0].name === '.' ? '.' : `.${zns[0].name}` }));
      }
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => { load().catch(console.error); }, []);

  const filtered = records.filter(r =>
    (typeFilter === 'ALL' || r.record_type === typeFilter) &&
    `${r.name} ${r.value}`.toLowerCase().includes(query.toLowerCase())
  );

  async function handleAdd() {
    let finalName = form.name;
    if (!form.is_dev && form.zoneSuffix && form.zoneSuffix !== '.') {
      finalName = form.name ? `${form.name}${form.zoneSuffix}` : form.zoneSuffix.substring(1);
    }
    
    await api.createRecord({ ...form, name: finalName });
    setForm({ ...defaultForm, zoneSuffix: form.zoneSuffix });
    await load();
  }

  async function handleDelete(id: number) {
    if (!window.confirm('Delete this DNS record?')) return;
    await api.deleteRecord(id);
    await load();
  }

  return (
    <Page title="DNS Records" subtitle="Manage authoritative records across your configured zones.">
      <div className="toolbar">
        <input
          id="records-search"
          placeholder="Search records…"
          value={query}
          onChange={e => setQuery(e.target.value)}
          style={{ maxWidth: 420 }}
        />
        <select
          id="records-type-filter"
          value={typeFilter}
          onChange={e => setTypeFilter(e.target.value)}
          style={{ width: 130 }}
        >
          <option>ALL</option>
          {RECORD_TYPES.map(t => <option key={t}>{t}</option>)}
        </select>
        <div className="spacer" />
      </div>

      <div className="form-panel">
        <h3>Add record</h3>
        
        <div style={{ marginBottom: '1rem' }}>
          <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', cursor: 'pointer', width: 'fit-content' }}>
            <input 
              type="checkbox" 
              checked={form.is_dev} 
              onChange={e => setForm({ ...form, is_dev: e.target.checked })} 
            />
            <span><strong>Dev record (ephemeral)</strong> — bypasses zone rules, deleted on restart</span>
          </label>
        </div>

        <div className="form-grid">
          <div style={{ display: 'flex', gap: '0.25rem' }}>
            <input 
              id="record-name" 
              placeholder={form.is_dev ? "FQDN (e.g. google.com)" : "Subdomain (e.g. www)"} 
              value={form.name} 
              onChange={e => setForm({ ...form, name: e.target.value })} 
              style={{ flex: 1 }}
            />
            {!form.is_dev && zones.length > 0 && (
              <select 
                value={form.zoneSuffix} 
                onChange={e => setForm({ ...form, zoneSuffix: e.target.value })}
                style={{ width: 'auto', minWidth: '120px' }}
              >
                {zones.map(z => {
                  const suffix = z.name === '.' ? '.' : `.${z.name}`;
                  return <option key={z.id} value={suffix}>{suffix}</option>;
                })}
              </select>
            )}
          </div>
          <select id="record-type" value={form.record_type} onChange={e => setForm({ ...form, record_type: e.target.value })}>
            {RECORD_TYPES.map(t => <option key={t}>{t}</option>)}
          </select>
          <input id="record-value" placeholder="Value" value={form.value} onChange={e => setForm({ ...form, value: e.target.value })} />
          <input id="record-ttl" type="number" min="0" value={form.ttl} onChange={e => setForm({ ...form, ttl: Number(e.target.value) })} />
          <button id="record-add" className="primary" type="button" onClick={() => handleAdd().catch(e => alert(e.message))}>Add</button>
        </div>
        {!form.is_dev && zones.length === 0 && (
          <div className="error-banner" style={{ marginTop: '1rem', padding: '0.5rem' }}>
            No authoritative zones configured. Please <a href="/zones" style={{ color: 'inherit', textDecoration: 'underline' }}>add a zone</a> first, or check the "Dev record" box.
          </div>
        )}
      </div>

      <div className="card" style={{ overflow: 'auto' }}>
        <table>
          <thead>
            <tr>
              <th>Name</th><th>Type</th><th>Value</th><th>TTL</th><th>Priority</th><th />
            </tr>
          </thead>
          <tbody>
            {loading
              ? <tr><td colSpan={6}>Loading…</td></tr>
              : filtered.length === 0
                ? <tr><td colSpan={6}>No records found.</td></tr>
                : filtered.map(r => (
                  <tr key={r.id}>
                    <td className="mono">
                      {r.name}
                      {r.is_dev && <span className="badge" style={{ marginLeft: '0.5rem', backgroundColor: '#e2e8f0', color: '#475569' }}>⚗ dev</span>}
                    </td>
                    <td><span className="tag">{r.record_type}</span></td>
                    <td className="mono value-cell">{r.value}</td>
                    <td>{r.ttl}s</td>
                    <td>{r.priority ?? '—'}</td>
                    <td>
                      <button
                        className="danger-link"
                        type="button"
                        onClick={() => handleDelete(r.id).catch(e => alert(e.message))}
                      >
                        Delete
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
