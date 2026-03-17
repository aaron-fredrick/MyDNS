// ── state ──────────────────────────────────────────────────────────────────
let token = localStorage.getItem('mydns_token') || '';
let ws = null;
let statsInterval = null;
let priority = 'cloudflare_first';

// ── auth ───────────────────────────────────────────────────────────────────
async function login() {
  const username = document.getElementById('username').value.trim();
  const password = document.getElementById('password').value;
  const errEl = document.getElementById('login-error');
  errEl.style.display = 'none';
  try {
    const res = await fetch('/api/v1/auth/login', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        username,
        password
      })
    });
    if (!res.ok) {
      errEl.style.display = 'block';
      errEl.textContent = 'Invalid credentials';
      return;
    }
    const data = await res.json();
    token = data.token;
    localStorage.setItem('mydns_token', token);
    showApp();
  } catch (e) {
    errEl.style.display = 'block';
    errEl.textContent = 'Connection error';
  }
}

function logout() {
  localStorage.removeItem('mydns_token');
  token = '';
  if (ws) ws.close();
  if (statsInterval) clearInterval(statsInterval);
  document.getElementById('app').style.display = 'none';
  document.getElementById('login-page').style.display = 'flex';
}

function authHeaders() {
  return {
    'Authorization': 'Bearer ' + token,
    'Content-Type': 'application/json'
  };
}

// ── boot ───────────────────────────────────────────────────────────────────
window.onload = () => {
  if (token) showApp();
  document.getElementById('password').addEventListener('keydown', e => {
    if (e.key === 'Enter') login();
  });
};

function showApp() {
  document.getElementById('login-page').style.display = 'none';
  document.getElementById('app').style.display = 'flex';
  loadStats();
  statsInterval = setInterval(loadStats, 5000);
  connectWS();
  loadRecords();
  loadSettings();
}

// ── navigation ─────────────────────────────────────────────────────────────
function showSection(id, el) {
  document.querySelectorAll('.section').forEach(s => s.classList.remove('active'));
  document.querySelectorAll('nav a').forEach(a => a.classList.remove('active'));
  document.getElementById('section-' + id).classList.add('active');
  if (el) el.classList.add('active');
  if (id === 'records') loadRecords();
  if (id === 'cache') loadCache();
  if (id === 'settings') loadSettings();
}

// ── stats ──────────────────────────────────────────────────────────────────
async function loadStats() {
  try {
    const res = await fetch('/api/v1/stats', {
      headers: authHeaders()
    });
    if (!res.ok) return;
    const d = await res.json();
    document.getElementById('s-uptime').textContent = formatUptime(d.uptime_secs);
    document.getElementById('s-hits').textContent = d.cache_hits;
    document.getElementById('s-misses').textContent = d.cache_misses;
    document.getElementById('s-cache-size').textContent = d.cache_size;
    document.getElementById('s-records').textContent = d.record_count;
  } catch (e) {}
}

function formatUptime(s) {
  if (s < 60) return s + 's';
  if (s < 3600) return Math.floor(s / 60) + 'm ' + (s % 60) + 's';
  return Math.floor(s / 3600) + 'h ' + Math.floor((s % 3600) / 60) + 'm';
}

// ── WebSocket ──────────────────────────────────────────────────────────────
function connectWS() {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  ws = new WebSocket(`${proto}://${location.host}/ws`);
  ws.onopen = () => setWsBadge(true);
  ws.onclose = () => {
    setWsBadge(false);
    setTimeout(connectWS, 3000);
  };
  ws.onerror = () => setWsBadge(false);
  ws.onmessage = e => appendLog(e.data);
}

function setWsBadge(connected) {
  ['ws-badge-dash', 'ws-badge-logs'].forEach(id => {
    const el = document.getElementById(id);
    if (el) {
      el.className = 'ws-badge ' + (connected ? 'connected' : 'disconnected');
      el.innerHTML = `<span class="ws-dot"></span>${connected ? 'Live' : 'Disconnected'}`;
    }
  });
}

function addLogLine(text) {
  const ts = new Date().toLocaleTimeString();
  const line = document.createElement('div');

  // Auto-detect type for row and tag styling
  const type = classifyLog(text);
  line.className = 'log-line ' + type;

  let processedText = escHtml(text);
  // Wrap tags like [UPSTREAM] in a span.tag if they exist at the start
  processedText = processedText.replace(/^(\[[A-Z\s]+\])/, '<span class="tag">$1</span>');

  line.innerHTML = `<span class="ts">${ts}</span>${processedText}`;

  ['log-panel-dash', 'log-panel-logs'].forEach(id => {
    const p = document.getElementById(id);
    if (p) {
      p.appendChild(line.cloneNode(true));
      p.scrollTop = p.scrollHeight;
      while (p.children.length > 500) p.removeChild(p.firstChild);
    }
  });
}

function appendLog(text) {
  addLogLine(text);
}

function classifyLog(t) {
  if (t.includes('[CACHE HIT]')) return 'hit';
  if (t.includes('[DB HIT]')) return 'hit-db';
  if (t.includes('[UPSTREAM]')) return 'upstream';
  if (t.includes('[NXDOMAIN]')) return 'nxdomain';
  if (t.includes('[CRUD]')) return 'crud';
  if (t.includes('[AUTH]')) return 'auth';
  if (t.includes('[SPECIAL]')) return 'special';
  return 'info';
}

function clearLogs() {
  ['log-panel-dash', 'log-panel-logs'].forEach(id => {
    document.getElementById(id).innerHTML = '';
  });
}

function escHtml(t) {
  const d = document.createElement('div');
  d.textContent = t;
  return d.innerHTML;
}

// ── Records ────────────────────────────────────────────────────────────────
let records = [];

async function loadRecords() {
  try {
    const res = await fetch('/api/v1/records', {
      headers: authHeaders()
    });
    if (!res.ok) return;
    const d = await res.json();
    records = d.records;
    renderRecords();
  } catch (e) {}
}

function renderRecords() {
  const tbody = document.getElementById('records-tbody');
  document.getElementById('record-count-label').textContent = `${records.length} record(s)`;

  const rows = [`<tr style="background:rgba(124,106,247,.05)">
        <td style="font-family:monospace;font-size:.85rem">mydns.local</td>
        <td><span class="badge" style="background:rgba(255,255,255,.1);color:var(--muted)">SYSTEM</span></td>
        <td style="font-family:monospace;font-size:.85rem;color:var(--accent2)">[Context Aware]</td>
        <td>60s</td>
        <td>—</td>
        <td><span style="color:var(--muted);font-size:.75rem;font-style:italic">Protected</span></td>
      </tr>`];

  records.forEach(r => {
    rows.push(`
        <tr>
          <td style="font-family:monospace;font-size:.85rem">${escHtml(r.name)}</td>
          <td><span class="badge badge-${r.record_type}">${r.record_type}</span></td>
          <td style="font-family:monospace;font-size:.85rem">${escHtml(r.value)}</td>
          <td>${r.ttl}s</td>
          <td>${r.priority ?? '—'}</td>
          <td>
            <div style="display:flex;gap:.4rem">
              <button class="btn-add" style="padding:.3rem .6rem;background:rgba(255,255,255,.06);color:var(--text);font-size:.8rem" onclick="openEditModal(${r.id})">Edit</button>
              <button class="btn-del" onclick="deleteRecord(${r.id})">Delete</button>
            </div>
          </td>
        </tr>`);
  });

  tbody.innerHTML = rows.join('');
}

// ── Modal Logic ──────────────────────────────────────────────────────────
function openAddModal() {
  document.getElementById('modal-title').textContent = 'Add DNS Record';
  document.getElementById('btn-save-record').textContent = 'Add Record';
  document.getElementById('m-id').value = '';
  closeModal(); // Reset fields
  document.getElementById('modal').classList.add('open');
}

function openEditModal(id) {
  const r = records.find(rec => rec.id === id);
  if (!r) return;
  document.getElementById('modal-title').textContent = 'Edit DNS Record';
  document.getElementById('btn-save-record').textContent = 'Update Record';
  document.getElementById('m-id').value = id;
  document.getElementById('m-name').value = r.name;
  document.getElementById('m-type').value = r.record_type;
  document.getElementById('m-value').value = r.value;
  document.getElementById('m-ttl').value = r.ttl;
  if (r.record_type === 'MX') {
    document.getElementById('m-priority-group').style.display = 'block';
    document.getElementById('m-priority').value = r.priority || 10;
  } else {
    document.getElementById('m-priority-group').style.display = 'none';
  }
  updateRecordPlaceholder();
  document.getElementById('modal').classList.add('open');
}

function updateRecordPlaceholder() {
  const type = document.getElementById('m-type').value;
  const valInput = document.getElementById('m-value');
  const placeholders = {
    'A': '192.168.1.1',
    'AAAA': '2001:db8::1',
    'CNAME': 'target.example.com.',
    'MX': 'mail.example.com.'
  };
  valInput.placeholder = placeholders[type] || 'Value';
  document.getElementById('m-priority-group').style.display = type === 'MX' ? 'block' : 'none';
}

function closeModal() {
  document.getElementById('modal').classList.remove('open');
  ['m-name', 'm-value'].forEach(id => document.getElementById(id).value = '');
  document.getElementById('m-ttl').value = '300';
  document.getElementById('m-type').value = 'A';
  document.getElementById('m-priority-group').style.display = 'none';
}

document.addEventListener('DOMContentLoaded', () => {
  const mType = document.getElementById('m-type');
  if (mType) {
    mType.addEventListener('change', function() {
      document.getElementById('m-priority-group').style.display = this.value === 'MX' ? 'block' : 'none';
    });
  }
});

async function saveRecord() {
  const id = document.getElementById('m-id').value;
  const body = {
    name: document.getElementById('m-name').value.trim(),
    record_type: document.getElementById('m-type').value,
    value: document.getElementById('m-value').value.trim(),
    ttl: parseInt(document.getElementById('m-ttl').value),
  };
  if (body.record_type === 'MX') body.priority = parseInt(document.getElementById('m-priority').value);

  const url = id ? '/api/v1/records/' + id : '/api/v1/records';
  const method = id ? 'PUT' : 'POST';

  try {
    const res = await fetch(url, {
      method: method,
      headers: authHeaders(),
      body: JSON.stringify(body)
    });
    if (!res.ok) {
      const e = await res.json();
      showToast('Error: ' + e.error, true);
      return;
    }
    closeModal();
    loadRecords();
    showToast(id ? 'Record updated ✓' : 'Record added ✓');
  } catch (e) {
    showToast('Connection error', true);
  }
}

async function deleteRecord(id) {
  if (!confirm('Delete this record?')) return;
  try {
    const res = await fetch('/api/v1/records/' + id, {
      method: 'DELETE',
      headers: authHeaders()
    });
    if (!res.ok) {
      showToast('Delete failed', true);
      return;
    }
    loadRecords();
    showToast('Record deleted ✓');
  } catch (e) {}
}

// ── Cache ──────────────────────────────────────────────────────────────────
async function loadCache() {
  try {
    const res = await fetch('/api/v1/cache', {
      headers: authHeaders()
    });
    if (!res.ok) return;
    const data = await res.json();
    renderCache(data);
  } catch (e) {}
}

function renderCache(data) {
  const tbody = document.getElementById('cache-tbody');
  document.getElementById('cache-count-label').textContent = `${data.length} cached entry(s)`;
  if (!data.length) {
    tbody.innerHTML = '<tr><td colspan="5" class="no-data">Cache is empty.</td></tr>';
    return;
  }
  tbody.innerHTML = data.sort((a, b) => a.name.localeCompare(b.name)).map(c => `
      <tr>
        <td style="font-family:monospace;font-size:.85rem">${escHtml(c.name)}</td>
        <td><span class="badge badge-${c.record_type}">${c.record_type}</span></td>
        <td>${c.ttl_remaining}s</td>
        <td style="font-family:monospace;font-size:.82rem">${escHtml(c.values.join(', '))}</td>
        <td><button class="btn-del" onclick="deleteCacheEntry('${c.name}', '${c.record_type}')">Delete</button></td>
      </tr>`).join('');
}

async function deleteCacheEntry(name, type) {
  try {
    const res = await fetch(`/api/v1/cache/${name}/${type}`, {
      method: 'DELETE',
      headers: authHeaders()
    });
    if (!res.ok) {
      showToast('Delete failed', true);
      return;
    }
    loadCache();
    showToast('Cache entry deleted ✓');
  } catch (e) {}
}

async function clearCache() {
  if (!confirm('Clear the entire DNS cache?')) return;
  try {
    const res = await fetch('/api/v1/cache', {
      method: 'DELETE',
      headers: authHeaders()
    });
    if (!res.ok) {
      showToast('Clear failed', true);
      return;
    }
    loadCache();
    showToast('Cache cleared ✓');
  } catch (e) {}
}

// ── Settings ───────────────────────────────────────────────────────────────
async function loadSettings() {
  try {
    const res = await fetch('/api/v1/settings', {
      headers: authHeaders()
    });
    if (!res.ok) return;
    const d = await res.json();
    priority = d.resolver_priority;
    updatePriorityUI();
    document.getElementById('cf-dns').value = d.cloudflare_dns || '1.1.1.1:53';
    document.getElementById('router-dns').value = d.router_dns || '';
  } catch (e) {}
}

function setPriority(p) {
  priority = p;
  updatePriorityUI();
}

function updatePriorityUI() {
  const pCf = document.getElementById('prio-cf');
  const pRt = document.getElementById('prio-rt');
  if (pCf) pCf.classList.toggle('active', priority === 'cloudflare_first');
  if (pRt) pRt.classList.toggle('active', priority === 'router_first');
}

async function saveSettings() {
  const body = {
    resolver_priority: priority
  };
  const cf = document.getElementById('cf-dns').value.trim();
  const rt = document.getElementById('router-dns').value.trim();
  if (cf) body.cloudflare_dns = cf;
  if (rt) body.router_dns = rt;
  try {
    const res = await fetch('/api/v1/settings', {
      method: 'PUT',
      headers: authHeaders(),
      body: JSON.stringify(body)
    });
    if (!res.ok) {
      const e = await res.json();
      showToast('Error: ' + e.error, true);
      return;
    }
    showToast('Settings saved ✓');
  } catch (e) {
    showToast('Connection error', true);
  }
}

// ── toast ──────────────────────────────────────────────────────────────────
function showToast(msg, isError = false) {
  const t = document.getElementById('toast');
  if (t) {
    t.textContent = msg;
    t.style.background = isError ? 'var(--danger)' : 'var(--success)';
    t.style.color = isError ? '#fff' : '#0a2218';
    t.classList.add('show');
    setTimeout(() => t.classList.remove('show'), 2800);
  }
}

// Initial UI state
setTimeout(updatePriorityUI, 100);
