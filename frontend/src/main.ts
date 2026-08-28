import './styles.css';
import { escapeHtml, formatUtcOffset, relativeTime } from './utils';

const SLUG = 'webhook-quiet-hours';
const LICENSE_KEY = `sb_license:${SLUG}`;
const VERDICT_KEY = `sb_license_verdict:${SLUG}`;
const BILLING_BASE = import.meta.env.VITE_BILLING_BASE || 'https://api.sociobot.in';
const app = document.querySelector<HTMLDivElement>('#app')!;

type Summary = { endpoints: number; fingerprints: number; events_today: number; pending: number; compressed: number; high_unacknowledged: number };
type Endpoint = { id: number; slug: string; name: string; signature_required: boolean; created_at: string };
type Fingerprint = { fingerprint: string; endpoint_id: number; endpoint_name: string; event_type: string; first_seen: string; last_seen: string; total_count: number; pending_count: number; severity: 'normal' | 'high' | 'ignored'; target_minutes: number; acknowledged_at: string | null; overdue: boolean };
type Settings = { quiet_start: string; quiet_end: string; utc_offset_minutes: number; digest_minutes: number; retention_days: number; notification_configured: boolean; notification_url: string; escalation_url: string; last_delivery_error: string | null };
type Model = { summary: Summary; endpoints: Endpoint[]; fingerprints: Fingerprint[]; settings: Settings };

let token = sessionStorage.getItem('qh_admin_token') || '';
let model: Model | null = null;
let activeView: 'observations' | 'aliases' | 'settings' = 'observations';
let licenseUnlocked = cachedLicenseValid();
let statusTimer = 0;

function cachedLicenseValid(): boolean {
  try {
    const cached = JSON.parse(localStorage.getItem(VERDICT_KEY) || '{}') as { valid?: boolean; checked_at?: number };
    return cached.valid === true && Date.now() - (cached.checked_at || 0) < 86_400_000;
  } catch { return false; }
}

async function processLicense(): Promise<void> {
  const url = new URL(location.href);
  const returned = url.searchParams.get('license');
  if (returned) {
    localStorage.setItem(LICENSE_KEY, returned);
    url.searchParams.delete('license');
    history.replaceState({}, '', `${url.pathname}${url.search}${url.hash}`);
  }
  const license = localStorage.getItem(LICENSE_KEY);
  if (!license) return;
  let checkedAt = 0;
  try { checkedAt = JSON.parse(localStorage.getItem(VERDICT_KEY) || '{}').checked_at || 0; } catch { /* verify */ }
  if (!returned && Date.now() - checkedAt < 86_400_000) return;
  try {
    const response = await fetch(`${BILLING_BASE}/api/v1/products/${SLUG}/verify?license=${encodeURIComponent(license)}`);
    const verdict = await response.json() as { valid: boolean; reason: string };
    localStorage.setItem(VERDICT_KEY, JSON.stringify({ valid: verdict.valid, reason: verdict.reason, checked_at: Date.now() }));
    licenseUnlocked = verdict.valid;
    if (!verdict.valid) showStatus('License no longer active. Free features remain available.', 'warning');
    if (token && model) renderDashboard();
  } catch {
    // A failed background verification never blocks the free experience or a cached unlock.
  }
}

async function api<T>(path: string, options: RequestInit = {}): Promise<T> {
  if (!navigator.onLine) throw new Error('You appear to be offline. Reconnect to reach the self-hosted receiver.');
  const response = await fetch(`/api${path}`, {
    ...options,
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}`, ...(options.headers || {}) },
  });
  if (response.status === 401) {
    sessionStorage.removeItem('qh_admin_token'); token = '';
    throw new Error('That admin token was not accepted. Check ADMIN_TOKEN on the server.');
  }
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: 'The server returned an unexpected response.' })) as { error?: string };
    throw new Error(body.error || 'The request could not be completed.');
  }
  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}

function shell(content: string): string {
  return `<header class="site-header"><a class="brand" href="/" aria-label="Webhook Quiet Hours home"><span aria-hidden="true">✦</span> Webhook Quiet Hours</a><button id="theme-toggle" class="icon-button" type="button" aria-label="Toggle color theme"><span aria-hidden="true">◐</span></button></header>${content}<footer><p>Self-hosted. No telemetry. Payloads encrypted at rest.</p><nav aria-label="Legal"><a href="/privacy">Privacy</a><a href="/terms">Terms</a><span>Original AI-generated botanical plate</span></nav></footer><div id="live-status" class="toast" role="status" aria-live="polite"></div>`;
}

function setupTheme(): void {
  const saved = localStorage.getItem('qh_theme');
  if (saved) document.documentElement.dataset.theme = saved;
  document.querySelector('#theme-toggle')?.addEventListener('click', () => {
    const next = document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = next;
    localStorage.setItem('qh_theme', next);
  });
}

function renderLanding(error = ''): void {
  document.title = 'Webhook Quiet Hours — compress webhook noise';
  app.innerHTML = shell(`<main id="main" class="landing"><section class="hero"><div class="hero-copy"><p class="eyebrow">A field guide for incoming noise</p><h1>Let webhook failures break the quiet. Not every webhook.</h1><p class="lede">Receive internal webhooks, press repeats into stable fingerprints, and send one digest—while high-severity signals still escalate on time.</p><form id="unlock-form" class="token-form"><label for="admin-token">Server admin token</label><div class="input-action"><input id="admin-token" name="token" type="password" required autocomplete="current-password" aria-describedby="token-help token-error"><button class="primary" type="submit">Open field log</button></div><p id="token-help" class="help">Read it from <code>data/admin-token</code> on the server, or supply <code>ADMIN_TOKEN</code>. It stays in this browser tab.</p><p id="token-error" class="form-error" role="alert">${escapeHtml(error)}</p></form><div class="hero-notes" aria-label="Core capabilities"><span>Signed aliases</span><span>Encrypted payloads</span><span>Quiet-hour digests</span></div></div><figure class="specimen"><picture><source media="(max-width: 600px)" srcset="/assets/moon-bloom-480.webp"><img src="/assets/moon-bloom-820.webp" width="820" height="820" alt="Field-guide illustration of many seed pods converging on one red terminal berry" fetchpriority="high" decoding="async"></picture><figcaption><span>Specimen 01</span> Many observations. One signal.</figcaption></figure></section><section class="method" aria-labelledby="method-title"><p class="margin-number">01—03</p><div><h2 id="method-title">Observe, classify, compress.</h2><ol><li><strong>Receive</strong><span>Give each provider a private alias and optional HMAC secret.</span></li><li><strong>Press</strong><span>Group matching event shape, type, and failure status into one fingerprint.</span></li><li><strong>Signal</strong><span>Digest normal repeats; immediately deliver high-severity fingerprints with one review link.</span></li></ol></div></section></main>`);
  setupTheme();
  document.querySelector<HTMLFormElement>('#unlock-form')?.addEventListener('submit', async (event) => {
    event.preventDefault();
    token = new FormData(event.currentTarget as HTMLFormElement).get('token')?.toString().trim() || '';
    sessionStorage.setItem('qh_admin_token', token);
    await loadDashboard();
  });
}

function renderLoading(): void {
  app.innerHTML = shell(`<main id="main" class="loading-view" aria-busy="true"><p class="eyebrow">Opening the field log</p><h1>Reading observations…</h1><div class="loading-lines" aria-hidden="true"><i></i><i></i><i></i></div></main>`);
  setupTheme();
}

async function loadDashboard(): Promise<void> {
  renderLoading();
  try {
    const [summary, endpoints, fingerprints, settings] = await Promise.all([
      api<Summary>('/summary'), api<Endpoint[]>('/endpoints'), api<Fingerprint[]>('/fingerprints'), api<Settings>('/settings'),
    ]);
    model = { summary, endpoints, fingerprints, settings };
    renderDashboard();
    const focus = new URL(location.href).searchParams.get('fingerprint');
    if (focus) void openFingerprint(focus);
  } catch (error) { renderLanding(error instanceof Error ? error.message : 'Could not reach the receiver.'); }
}

function renderDashboard(): void {
  if (!model) return;
  const tab = (id: typeof activeView, label: string) => `<button class="tab ${activeView === id ? 'active' : ''}" role="tab" aria-selected="${activeView === id}" data-view="${id}" type="button">${label}</button>`;
  app.innerHTML = shell(`<main id="main" class="app-shell"><div class="app-heading"><div><p class="eyebrow">Live field log</p><h1>Webhook observations</h1></div><div class="connection"><span class="pulse" aria-hidden="true"></span>Receiver online</div></div><section class="summary-strip" aria-label="Current summary"><article><span>Observed today</span><strong>${model.summary.events_today}</strong></article><article><span>Fingerprints</span><strong>${model.summary.fingerprints}</strong></article><article><span>Repeats compressed</span><strong>${model.summary.compressed}</strong></article><article class="${model.summary.high_unacknowledged ? 'attention' : ''}"><span>High awaiting ack</span><strong>${model.summary.high_unacknowledged}</strong></article></section><div class="workbench"><nav class="tabs" role="tablist" aria-label="Field log sections">${tab('observations', 'Observations')}${tab('aliases', `Aliases · ${model.endpoints.length}`)}${tab('settings', 'Quiet rules')}</nav><section id="panel" class="panel" role="tabpanel"></section></div></main>`);
  setupTheme(); setupTabs(); renderActivePanel();
  document.querySelector<HTMLButtonElement>('.tab.active')?.focus({ preventScroll: true });
}

function setupTabs(): void {
  document.querySelectorAll<HTMLButtonElement>('[data-view]').forEach((button) => button.addEventListener('click', () => {
    activeView = button.dataset.view as typeof activeView; renderDashboard();
  }));
}

function renderActivePanel(): void {
  if (activeView === 'observations') renderObservations();
  if (activeView === 'aliases') renderAliases();
  if (activeView === 'settings') renderSettings();
}

function renderObservations(): void {
  if (!model) return;
  const panel = document.querySelector<HTMLElement>('#panel')!;
  if (!model.fingerprints.length) {
    panel.innerHTML = `<div class="empty-state"><span class="pressed-leaf" aria-hidden="true">⌁</span><h2>No specimens pressed yet</h2><p>Create an alias, send a webhook, and repeat payload shapes will collect here as one fingerprint.</p><button class="primary" id="empty-alias" type="button">Create the first alias</button></div>`;
    document.querySelector('#empty-alias')?.addEventListener('click', () => { activeView = 'aliases'; renderDashboard(); }); return;
  }
  panel.innerHTML = `<div class="panel-heading"><div><h2>Fingerprint ledger</h2><p>${model.summary.pending} observations are waiting for the next signal.</p></div><div class="panel-actions"><button id="export" class="secondary" type="button">Export CSV</button><button id="refresh" class="secondary" type="button">Refresh</button></div></div><ol class="ledger">${model.fingerprints.map((f, index) => fingerprintRow(f, index)).join('')}</ol>`;
  document.querySelector('#refresh')?.addEventListener('click', loadDashboard);
  document.querySelector('#export')?.addEventListener('click', exportCsv);
  document.querySelectorAll<HTMLButtonElement>('[data-fingerprint]').forEach((button) => button.addEventListener('click', () => void openFingerprint(button.dataset.fingerprint!)));
  document.querySelectorAll<HTMLButtonElement>('[data-ack]').forEach((button) => button.addEventListener('click', () => void acknowledge(button.dataset.ack!)));
}

function fingerprintRow(f: Fingerprint, index: number): string {
  const label = f.severity === 'high' ? (f.overdue ? 'High · overdue' : 'High') : f.severity === 'ignored' ? 'Ignored' : 'Digest';
  return `<li class="fingerprint ${f.severity} ${f.overdue ? 'overdue' : ''}"><span class="specimen-number">${String(index + 1).padStart(2, '0')}</span><button class="fingerprint-main" type="button" data-fingerprint="${f.fingerprint}" aria-label="Inspect ${escapeHtml(f.event_type)} fingerprint"><span><strong>${escapeHtml(f.event_type)}</strong><small>${escapeHtml(f.endpoint_name)} · <code>${f.fingerprint}</code></small></span><span class="counts"><b>${f.total_count}</b><small>observations</small></span><span class="last-seen"><b>${relativeTime(f.last_seen)}</b><small>last seen</small></span><span class="severity-tag ${f.severity}">${label}</span></button>${f.severity === 'high' && !f.acknowledged_at ? `<button class="ack-button" data-ack="${f.fingerprint}" type="button">Acknowledge</button>` : ''}<div id="detail-${f.fingerprint}" class="detail" hidden></div></li>`;
}

async function openFingerprint(fp: string): Promise<void> {
  const container = document.querySelector<HTMLElement>(`#detail-${CSS.escape(fp)}`);
  if (!container) return;
  if (!container.hidden) { container.hidden = true; return; }
  container.hidden = false; container.innerHTML = '<p class="detail-loading">Decrypting the latest retained payload…</p>';
  try {
    const detail = await api<{ event_type: string; payload: unknown; received_at: string; signature_valid: boolean }>(`/fingerprints/${fp}`);
    const current = model?.fingerprints.find((f) => f.fingerprint === fp)!;
    container.innerHTML = `<div class="detail-grid"><div><p class="detail-label">Latest payload · ${escapeHtml(new Date(detail.received_at).toLocaleString())}</p><pre tabindex="0">${escapeHtml(JSON.stringify(detail.payload, null, 2))}</pre><p class="signature-ok">✓ Alias key${model?.endpoints.find((e) => e.id === current.endpoint_id)?.signature_required ? ' and HMAC signature' : ''} verified</p></div><form class="classification-form"><h3>Classification rule</h3><label for="severity-${fp}">Signal policy</label><select id="severity-${fp}" name="severity"><option value="normal" ${current.severity === 'normal' ? 'selected' : ''}>Include in digest</option><option value="high" ${current.severity === 'high' ? 'selected' : ''}>Escalate immediately</option><option value="ignored" ${current.severity === 'ignored' ? 'selected' : ''}>Record only</option></select><label for="target-${fp}">Acknowledge target (minutes)</label><input id="target-${fp}" name="target_minutes" type="number" min="1" max="1440" value="${current.target_minutes}"><button class="primary" type="submit">Save rule</button></form></div>`;
    container.querySelector<HTMLFormElement>('form')?.addEventListener('submit', async (event) => {
      event.preventDefault(); const data = new FormData(event.currentTarget as HTMLFormElement);
      try { await api(`/fingerprints/${fp}`, { method: 'PATCH', body: JSON.stringify({ severity: data.get('severity'), target_minutes: Number(data.get('target_minutes')) }) }); showStatus('Classification rule saved.'); await loadDashboard(); } catch (error) { showStatus(message(error), 'error'); }
    });
  } catch (error) { container.innerHTML = `<p class="form-error" role="alert">${escapeHtml(message(error))}</p>`; }
}

async function acknowledge(fp: string): Promise<void> {
  try { await api(`/fingerprints/${fp}/ack`, { method: 'POST' }); showStatus('High-severity fingerprint acknowledged.'); await loadDashboard(); } catch (error) { showStatus(message(error), 'error'); }
}

function renderAliases(): void {
  if (!model) return;
  const paidLimit = !licenseUnlocked && model.endpoints.length >= 1;
  document.querySelector<HTMLElement>('#panel')!.innerHTML = `<div class="panel-heading"><div><h2>Signed endpoint aliases</h2><p>Each alias has an unguessable receiver key. HMAC secrets are shown once.</p></div><button class="primary" id="new-alias" type="button" ${paidLimit ? 'aria-describedby="alias-limit"' : ''}>Create alias</button></div>${paidLimit ? `<p id="alias-limit" class="upgrade-note"><strong>Free field kit:</strong> one alias is active. The one-time Field Station unlock adds unlimited aliases and 90-day retention. <a href="${BILLING_BASE}/api/v1/products/${SLUG}/checkout">Buy for $39 once</a>.</p>` : ''}<div class="alias-list">${model.endpoints.length ? model.endpoints.map((endpoint) => `<article><div><span class="alias-mark" aria-hidden="true">⌘</span><h3>${escapeHtml(endpoint.name)}</h3><p><code>/hooks/${escapeHtml(endpoint.slug)}</code></p></div><span class="signature-ok">${endpoint.signature_required ? '✓ HMAC required' : '✓ Private alias key'}</span><button class="danger-quiet" type="button" data-delete-alias="${endpoint.id}" data-alias-name="${escapeHtml(endpoint.name)}">Delete</button></article>`).join('') : '<p class="empty-inline">No aliases yet. Create one to receive your first webhook.</p>'}</div><section class="paid-card" aria-labelledby="paid-title"><div><p class="eyebrow">One-time unlock · $39</p><h3 id="paid-title">Field Station</h3><p>Unlimited aliases, up to 90-day retention, and every future self-hosted release. Core signing, escalation, and export stay free.</p></div>${licenseUnlocked ? '<span class="license-active">✓ License active</span>' : `<div class="license-actions"><a class="primary link-button" href="${BILLING_BASE}/api/v1/products/${SLUG}/checkout">Buy once</a><button id="restore" class="secondary" type="button">Restore license</button></div>`}</section>`;
  document.querySelector('#new-alias')?.addEventListener('click', () => paidLimit ? document.querySelector<HTMLElement>('.paid-card')?.scrollIntoView({ behavior: 'smooth' }) : openAliasDialog());
  document.querySelectorAll<HTMLButtonElement>('[data-delete-alias]').forEach((button) => button.addEventListener('click', () => void deleteAlias(Number(button.dataset.deleteAlias), button.dataset.aliasName!)));
  document.querySelector('#restore')?.addEventListener('click', openRestoreDialog);
}

function ensureDialog(): HTMLDialogElement {
  let dialog = document.querySelector<HTMLDialogElement>('#modal');
  if (!dialog) { dialog = document.createElement('dialog'); dialog.id = 'modal'; document.body.append(dialog); }
  return dialog;
}

function openAliasDialog(): void {
  const dialog = ensureDialog();
  dialog.innerHTML = `<form id="alias-form"><div class="dialog-heading"><div><p class="eyebrow">New specimen inlet</p><h2>Create an alias</h2></div><button class="icon-button close" type="button" aria-label="Close dialog">×</button></div><label for="alias-name">Provider or system name</label><input id="alias-name" name="name" required minlength="2" maxlength="60" autocomplete="off"><label class="check-row"><input name="require_signature" type="checkbox" checked><span><strong>Require HMAC-SHA256</strong><small>Verify <code>X-Hub-Signature-256</code> or <code>X-Webhook-Signature</code>.</small></span></label><p class="form-error" role="alert"></p><div class="dialog-actions"><button class="secondary cancel" type="button">Cancel</button><button class="primary" type="submit">Create signed alias</button></div></form>`;
  dialog.showModal();
  dialog.querySelector<HTMLInputElement>('#alias-name')?.focus();
  dialog.querySelector('.close')?.addEventListener('click', () => dialog.close());
  dialog.querySelector('.cancel')?.addEventListener('click', () => dialog.close());
  dialog.querySelector<HTMLFormElement>('form')?.addEventListener('submit', async (event) => {
    event.preventDefault(); const form = event.currentTarget as HTMLFormElement; const data = new FormData(form); const submit = form.querySelector<HTMLButtonElement>('button[type=submit]')!; submit.disabled = true; submit.textContent = 'Planting alias…';
    try {
      const created = await api<{ hook_url: string; signing_secret?: string; name: string }>('/endpoints', { method: 'POST', body: JSON.stringify({ name: data.get('name'), require_signature: data.get('require_signature') === 'on' }) });
      dialog.innerHTML = `<div class="dialog-heading"><div><p class="eyebrow">Created · copy now</p><h2>${escapeHtml(created.name)}</h2></div><button class="icon-button close" type="button" aria-label="Close dialog">×</button></div><p>The receiver key and signing secret are not shown again.</p><label for="hook-result">Receiver URL</label><div class="copy-field"><input id="hook-result" readonly value="${escapeHtml(created.hook_url)}"><button type="button" data-copy="hook-result">Copy</button></div>${created.signing_secret ? `<label for="secret-result">HMAC secret</label><div class="copy-field"><input id="secret-result" readonly value="${escapeHtml(created.signing_secret)}"><button type="button" data-copy="secret-result">Copy</button></div>` : ''}<p class="help">Sign the exact raw request body with HMAC-SHA256 and send its hex digest as <code>sha256=&lt;digest&gt;</code>.</p><div class="dialog-actions"><button class="primary done" type="button">I saved these details</button></div>`;
      dialog.querySelectorAll<HTMLButtonElement>('[data-copy]').forEach((button) => button.addEventListener('click', () => void copyField(button.dataset.copy!, button)));
      dialog.querySelector('.close')?.addEventListener('click', () => { dialog.close(); void loadDashboard(); });
      dialog.querySelector('.done')?.addEventListener('click', () => { dialog.close(); void loadDashboard(); });
    } catch (error) { form.querySelector<HTMLElement>('.form-error')!.textContent = message(error); submit.disabled = false; submit.textContent = 'Create signed alias'; }
  });
}

async function copyField(id: string, button: HTMLButtonElement): Promise<void> {
  const input = document.querySelector<HTMLInputElement>(`#${id}`)!; await navigator.clipboard.writeText(input.value); button.textContent = 'Copied'; showStatus('Copied to clipboard.');
}

async function deleteAlias(id: number, name: string): Promise<void> {
  if (!confirm(`Delete “${name}” and all of its retained payloads and fingerprints? This cannot be undone.`)) return;
  try { await api(`/endpoints/${id}`, { method: 'DELETE' }); showStatus(`${name} and its retained observations were deleted.`); await loadDashboard(); } catch (error) { showStatus(message(error), 'error'); }
}

function openRestoreDialog(): void {
  const dialog = ensureDialog(); dialog.innerHTML = `<form id="restore-form"><div class="dialog-heading"><div><p class="eyebrow">Move devices</p><h2>Restore a license</h2></div><button class="icon-button close" type="button" aria-label="Close dialog">×</button></div><label for="license-token">License token</label><textarea id="license-token" name="license" required rows="4" spellcheck="false"></textarea><p class="help">The token is stored only in this browser and verified with Sociobot once per day.</p><p class="form-error" role="alert"></p><div class="dialog-actions"><button class="secondary cancel" type="button">Cancel</button><button class="primary" type="submit">Verify license</button></div></form>`; dialog.showModal(); dialog.querySelector<HTMLInputElement>('#license-token')?.focus(); dialog.querySelector('.close')?.addEventListener('click', () => dialog.close()); dialog.querySelector('.cancel')?.addEventListener('click', () => dialog.close()); dialog.querySelector<HTMLFormElement>('form')?.addEventListener('submit', async (event) => { event.preventDefault(); const form = event.currentTarget as HTMLFormElement; const value = new FormData(form).get('license')?.toString().trim(); if (!value) return; localStorage.setItem(LICENSE_KEY, value); localStorage.removeItem(VERDICT_KEY); await processLicense(); if (licenseUnlocked) { dialog.close(); showStatus('Field Station license restored.'); renderDashboard(); } else form.querySelector<HTMLElement>('.form-error')!.textContent = 'That license could not be verified as active.'; });
}

function renderSettings(): void {
  if (!model) return; const s = model.settings; const maxRetention = licenseUnlocked ? 90 : 7;
  const offsetOptions = Array.from({ length: 27 }, (_, i) => (i - 12) * 60).map((value) => `<option value="${value}" ${s.utc_offset_minutes === value ? 'selected' : ''}>${formatUtcOffset(value)}</option>`).join('');
  document.querySelector<HTMLElement>('#panel')!.innerHTML = `<div class="panel-heading"><div><h2>Quiet rules</h2><p>Normal fingerprints wait; high severity always breaks through.</p></div><button id="send-digest" class="secondary" type="button">Send digest now</button></div>${s.last_delivery_error ? `<div class="alert error" role="alert"><strong>Last delivery failed</strong><span>${escapeHtml(s.last_delivery_error)} Check the notification URL and try a manual digest.</span></div>` : ''}<form id="settings-form" class="settings-form"><fieldset><legend>Quiet window</legend><div class="field-pair"><label>Starts<input name="quiet_start" type="time" value="${s.quiet_start}" required></label><label>Ends<input name="quiet_end" type="time" value="${s.quiet_end}" required></label></div><label>Timezone<select name="utc_offset_minutes">${offsetOptions}</select></label><p class="help">Normal observations remain pending during this window. High severity still sends immediately.</p></fieldset><fieldset><legend>Digest and deletion</legend><label>Digest interval<select name="digest_minutes"><option value="15" ${s.digest_minutes === 15 ? 'selected' : ''}>Every 15 minutes</option><option value="30" ${s.digest_minutes === 30 ? 'selected' : ''}>Every 30 minutes</option><option value="60" ${s.digest_minutes === 60 ? 'selected' : ''}>Every hour</option><option value="240" ${s.digest_minutes === 240 ? 'selected' : ''}>Every 4 hours</option><option value="1440" ${s.digest_minutes === 1440 ? 'selected' : ''}>Daily</option></select></label><label>Delete retained payloads after<select name="retention_days"><option value="1" ${s.retention_days === 1 ? 'selected' : ''}>1 day</option><option value="7" ${s.retention_days === 7 ? 'selected' : ''}>7 days</option>${licenseUnlocked ? `<option value="30" ${s.retention_days === 30 ? 'selected' : ''}>30 days</option><option value="90" ${s.retention_days === 90 ? 'selected' : ''}>90 days</option>` : ''}</select></label><p class="help">Free retention is 7 days. Aggregated counts remain after individual payloads are deleted.${maxRetention > 7 ? ' Field Station extends this to 90 days.' : ''}</p></fieldset><fieldset class="wide"><legend>One notification destination</legend><label for="notification-url">Slack-compatible incoming webhook URL</label><input id="notification-url" name="notification_url" type="url" value="${escapeHtml(s.notification_url)}" placeholder="https://hooks.example.net/services/…" autocomplete="off"><label for="escalation-url">Single review link <span>(optional)</span></label><input id="escalation-url" name="escalation_url" type="url" value="${escapeHtml(s.escalation_url)}" placeholder="https://your-runbook.example/incidents" autocomplete="url"><p class="help">If left blank, messages link back to this field log. URLs are encrypted in SQLite.</p></fieldset><p class="form-error wide" role="alert"></p><div class="form-actions wide"><button class="primary" type="submit">Save quiet rules</button></div></form>`;
  document.querySelector<HTMLFormElement>('#settings-form')?.addEventListener('submit', saveSettings);
  document.querySelector('#send-digest')?.addEventListener('click', sendDigest);
}

async function saveSettings(event: Event): Promise<void> {
  event.preventDefault(); const form = event.currentTarget as HTMLFormElement; const data = new FormData(form); const body = Object.fromEntries(data.entries()) as Record<string, string>;
  try { await api('/settings', { method: 'PUT', body: JSON.stringify({ ...body, utc_offset_minutes: Number(body.utc_offset_minutes), digest_minutes: Number(body.digest_minutes), retention_days: Number(body.retention_days) }) }); showStatus('Quiet rules saved.'); await loadDashboard(); } catch (error) { form.querySelector<HTMLElement>('.form-error')!.textContent = message(error); }
}

async function sendDigest(): Promise<void> {
  try { const result = await api<{ sent: number }>('/digest/send', { method: 'POST' }); showStatus(result.sent ? `Digest sent with ${result.sent} fingerprints.` : 'Nothing pending, so no digest was sent.'); await loadDashboard(); } catch (error) { showStatus(message(error), 'error'); }
}

async function exportCsv(): Promise<void> {
  try { const response = await fetch('/api/export.csv', { headers: { Authorization: `Bearer ${token}` } }); if (!response.ok) throw new Error('Export failed.'); const blob = await response.blob(); const url = URL.createObjectURL(blob); const link = document.createElement('a'); link.href = url; link.download = 'webhook-fingerprints.csv'; link.click(); URL.revokeObjectURL(url); showStatus('Fingerprint CSV exported.'); } catch (error) { showStatus(message(error), 'error'); }
}

function renderLegal(kind: 'privacy' | 'terms'): void {
  const privacy = kind === 'privacy'; document.title = `${privacy ? 'Privacy' : 'Terms'} — Webhook Quiet Hours`;
  const body = privacy ? `<p class="effective">Effective 27 August 2026</p><h2>Your instance, your data</h2><p>Webhook Quiet Hours is self-hosted. Your server stores endpoint aliases, event fingerprints, configuration, and retained webhook payloads in its local SQLite database. Payloads, signing secrets, and notification URLs are encrypted at rest with your <code>DATA_ENCRYPTION_KEY</code>.</p><h2>What leaves the instance</h2><p>Configured digests are sent only to the notification URL you provide. When you buy or verify a Field Station license, your browser contacts Sociobot’s billing API with the license token. Sociobot/Dodo acts as merchant of record. This product has no analytics, advertising, cross-tenant telemetry, third-party fonts, or tracking scripts.</p><h2>Deletion and browser storage</h2><p>Retention cleanup deletes individual payloads on your schedule. Deleting an alias removes its observations. The admin token is kept in session storage. A license token and daily verification verdict are kept in local storage and can be removed with your browser controls.</p>` : `<p class="effective">Effective 27 August 2026</p><h2>Use of the software</h2><p>Webhook Quiet Hours is provided under the MIT License. You are responsible for securing the server, setting strong keys, limiting access, and configuring webhook providers correctly. It is an aggregation and alerting aid, not a delivery, retry, or guaranteed incident-response service.</p><h2>One-time purchase</h2><p>The $39 Field Station purchase unlocks unlimited endpoint aliases, up to 90-day payload retention, and future self-hosted releases for this product. Sociobot/Dodo is the merchant of record and handles payment and refunds. A refund revokes the associated license. Core signature verification, high-severity behavior, and export remain available without purchase.</p><h2>Availability and liability</h2><p>The software is provided “as is,” without warranty. You should test notification delivery and maintain independent recovery procedures appropriate to your systems. To the extent permitted by law, the authors are not liable for missed, delayed, or misclassified webhook events.</p>`;
  app.innerHTML = shell(`<main id="main" class="legal"><p class="eyebrow">Field notes · legal</p><h1>${privacy ? 'Privacy' : 'Terms of use'}</h1>${body}<p><a href="/">Return to Webhook Quiet Hours</a></p></main>`); setupTheme();
}

function showStatus(text: string, type: 'info' | 'warning' | 'error' = 'info'): void {
  const toast = document.querySelector<HTMLElement>('#live-status'); if (!toast) return; clearTimeout(statusTimer); toast.textContent = text; toast.dataset.type = type; toast.classList.add('visible'); statusTimer = window.setTimeout(() => toast.classList.remove('visible'), 5000);
}
function message(error: unknown): string { return error instanceof Error ? error.message : 'The request could not be completed.'; }

window.addEventListener('offline', () => showStatus('Offline. New data and changes will resume after you reconnect.', 'warning'));
window.addEventListener('online', () => { showStatus('Back online. Refreshing observations.'); if (token) void loadDashboard(); });

const path = location.pathname.replace(/\/$/, '') || '/';
if (path === '/privacy' || path === '/terms') renderLegal(path.slice(1) as 'privacy' | 'terms');
else if (token) void loadDashboard(); else renderLanding();
void processLicense();
if ('serviceWorker' in navigator && import.meta.env.PROD) window.addEventListener('load', () => void navigator.serviceWorker.register('/sw.js'));
