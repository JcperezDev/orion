// src/lib/chat.ts
// Chat island module — owns the only mutable state in the UI layer.
// Runs entirely in the browser; no React, no framework runtime.
//
// Public API (used by App.astro and the command palette):
//   init()      — wire up event listeners and start loading initial state
//   setView(v)  — switch the main area between chat / memory / servers / settings
//   isReady()   — true once init() has finished wiring up
//
// The module also exposes a window.__orion_chat object so the palette
// (a separate <script> in App.astro) can call newSession / deleteSession
// / reloadSessions without sharing module scope.

import {
  streamChat,
  getProviders,
  getModels,
  listSessions,
  createSession,
  deleteSession,
  type ChatMessage,
  type ModelInfo,
  type ProviderInfo,
  type SessionInfo,
} from './api';

interface ChatState {
  messages: ChatMessage[];
  provider: string;
  model: string;
  sessionId: string;
  busy: boolean;
  sessions: SessionInfo[];
  providers: ProviderInfo[];
  models: ModelInfo[];
  tokensUsed: number;
  serverOk: boolean;
}

const state: ChatState = {
  messages: [],
  provider: '',
  model: '',
  sessionId: typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : Math.random().toString(36).slice(2),
  busy: false,
  sessions: [],
  providers: [],
  models: [],
  tokensUsed: 0,
  serverOk: false,
};

let ready = false;

function $(id: string): HTMLElement | null {
  return document.getElementById(id);
}

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attrs: Record<string, string> = {},
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  if (text !== undefined) node.textContent = text;
  return node;
}

// ── view switching ────────────────────────────────────────────────────

export function setView(view: string): void {
  document.documentElement.setAttribute('data-view', view);
  document.querySelectorAll<HTMLElement>('[data-view-panel]').forEach((p) => {
    if (p.getAttribute('data-view-panel') === view) p.setAttribute('data-active', '');
    else p.removeAttribute('data-active');
  });
  document.querySelectorAll<HTMLButtonElement>('nav.sidebar button[data-view]').forEach((b) => {
    if (b.getAttribute('data-view') === view) b.classList.add('active');
    else b.classList.remove('active');
  });
}

export function isReady(): boolean {
  return ready;
}

// ── chrome updates ────────────────────────────────────────────────────

function updateProviderModelDisplay(): void {
  setText('crumbs-info', `${state.provider}/${state.model || '—'}`);
  setText('provider-name', state.provider || '—');
  setText('model-name', state.model || '—');
  setText('meta-provider', state.provider || '—');
  setText('meta-model', state.model || '—');
  setText('session-id', state.sessionId.slice(0, 8));
  setText('meta-session', state.sessionId.slice(0, 8));
}

function updateStatus(): void {
  const status = $('server-status');
  if (status) status.setAttribute('data-online', state.serverOk ? 'true' : 'false');
  setText('server-status-text', state.serverOk ? 'online' : 'offline');
}

function updateTokensDisplay(): void {
  setText('tokens-count', Math.round(state.tokensUsed).toLocaleString());
  const bar = $('tokens-bar') as HTMLDivElement | null;
  if (bar) bar.style.width = `${Math.min(100, (state.tokensUsed / 200000) * 100)}%`;
}

function setText(id: string, text: string): void {
  const node = $(id);
  if (node) node.textContent = text;
}

// ── sidebar renderers ─────────────────────────────────────────────────

function renderProviderPicker(): void {
  const picker = $('provider-picker') as HTMLSelectElement | null;
  if (!picker) return;
  picker.innerHTML = '';
  for (const p of state.providers) {
    const opt = el('option', { value: p.id });
    opt.textContent = `${p.name} ${p.available ? '· online' : '· offline'}`;
    picker.appendChild(opt);
  }
  picker.value = state.provider;
  picker.disabled = state.providers.length === 0;
  picker.onchange = () => {
    state.provider = picker.value;
    const m = state.models.find((x) => x.providerId === state.provider);
    if (m) {
      state.model = m.modelId;
      renderModelPicker();
    }
    updateProviderModelDisplay();
  };
}

function renderModelPicker(): void {
  const picker = $('model-picker') as HTMLSelectElement | null;
  if (!picker) return;
  picker.innerHTML = '';
  const filtered = state.models.filter((m) => m.providerId === state.provider);
  if (filtered.length === 0) {
    const opt = el('option', { value: '' }, 'no models synced');
    picker.appendChild(opt);
    picker.disabled = true;
    return;
  }
  picker.disabled = false;
  for (const m of filtered) {
    const opt = el('option', { value: m.modelId });
    opt.textContent = `${m.displayName || m.modelId} ${m.isFree ? '·free' : ''}`;
    picker.appendChild(opt);
  }
  picker.value = state.model;
  picker.onchange = () => {
    state.model = picker.value;
    updateProviderModelDisplay();
  };
}

function renderSessionsList(): void {
  const list = $('sessions-list');
  if (!list) return;
  setText('sessions-count', String(state.sessions.length));
  list.innerHTML = '';
  if (state.sessions.length === 0) {
    list.appendChild(
      el('div', { class: 'sidebar-empty' }, 'No sessions yet. Send a message to start one.'),
    );
    return;
  }
  for (const s of state.sessions.slice(0, 10)) {
    const btn = el('button', {
      class: s.id === state.sessionId ? 'active' : '',
      title: `${s.provider}/${s.model} · ${s.message_count} msgs`,
    });
    const label = el('span', { class: 'session-label' }, s.title);
    const pill = el('span', { class: 'pill' }, String(s.message_count));
    btn.appendChild(label);
    btn.appendChild(pill);
    btn.addEventListener('click', () => selectSession(s.id));
    list.appendChild(btn);
  }
}

// ── session actions ───────────────────────────────────────────────────

function selectSession(id: string): void {
  state.sessionId = id;
  state.messages = [];
  renderMessages();
  renderSessionsList();
  updateProviderModelDisplay();
  setView('chat');
}

export async function newSession(): Promise<void> {
  try {
    const s = await createSession({ title: 'New session', provider: state.provider, model: state.model });
    state.sessionId = s.id;
    state.messages = [];
    await reloadSessions();
    renderMessages();
    updateProviderModelDisplay();
    setView('chat');
  } catch (e) {
    console.error('newSession failed', e);
  }
}

export async function deleteCurrentSession(): Promise<void> {
  if (!state.sessionId) return;
  try {
    await deleteSession(state.sessionId);
  } catch (e) {
    console.error('deleteSession failed', e);
  }
  state.sessionId = typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : Math.random().toString(36).slice(2);
  state.messages = [];
  await reloadSessions();
  renderMessages();
  updateProviderModelDisplay();
}

export async function reloadSessions(): Promise<void> {
  try {
    state.sessions = await listSessions();
  } catch {
    state.sessions = [];
  }
  renderSessionsList();
}

// ── chat messages ─────────────────────────────────────────────────────

function renderMessages(): void {
  const container = $('messages') as HTMLDivElement | null;
  if (!container) return;
  container.innerHTML = '';
  if (state.messages.length === 0) {
    const welcome = el('div', { class: 'msg system' });
    welcome.appendChild(el('div', { class: 'role' }, 'ORION'));
    welcome.appendChild(document.createTextNode(' Connected to '));
    welcome.appendChild(el('b', {}, state.provider || '—'));
    welcome.appendChild(document.createTextNode(' · '));
    welcome.appendChild(el('b', {}, state.model || '—'));
    welcome.appendChild(document.createTextNode('. Send a message to begin.'));
    container.appendChild(welcome);
  } else {
    for (const m of state.messages) {
      const msg = el('div', {
        class: `msg ${m.role}${m.streaming ? ' cursor-blink' : ''}`,
      });
      msg.appendChild(el('div', { class: 'role' }, m.role));
      const text = m.content || (m.streaming ? '…' : '');
      msg.appendChild(document.createTextNode(text));
      container.appendChild(msg);
    }
  }
  requestAnimationFrame(() => {
    container.scrollTo({ top: container.scrollHeight, behavior: 'smooth' });
  });
}

async function send(): Promise<void> {
  const input = $('chat-input') as HTMLTextAreaElement | null;
  if (!input) return;
  const text = input.value.trim();
  if (!text || state.busy) return;
  input.value = '';
  state.busy = true;
  const sendBtn = $('chat-send') as HTMLButtonElement | null;
  if (sendBtn) sendBtn.disabled = true;

  const userMsg: ChatMessage = {
    id: crypto.randomUUID(),
    role: 'user',
    content: text,
    createdAt: Date.now(),
  };
  const assistantId = crypto.randomUUID();
  state.messages.push(userMsg);
  state.messages.push({
    id: assistantId,
    role: 'assistant',
    content: '',
    streaming: true,
    createdAt: Date.now(),
  });
  renderMessages();

  try {
    const history = state.messages
      .filter((m) => !m.streaming && m.id !== assistantId)
      .map((m) => ({ role: m.role, content: m.content }));
    const gen = streamChat({
      provider: state.provider,
      model: state.model,
      message: text,
      sessionId: state.sessionId,
      history,
    });
    let acc = '';
    for await (const tok of gen) {
      acc += tok;
      state.tokensUsed += tok.length / 4;
      const m = state.messages.find((x) => x.id === assistantId);
      if (m) m.content = acc;
      updateTokensDisplay();
      renderMessages();
    }
    const m = state.messages.find((x) => x.id === assistantId);
    if (m) m.streaming = false;
    renderMessages();
  } catch (e) {
    const m = state.messages.find((x) => x.id === assistantId);
    if (m) {
      m.content = `[error] ${(e as Error).message}`;
      m.streaming = false;
    }
    renderMessages();
  } finally {
    state.busy = false;
    if (sendBtn) sendBtn.disabled = false;
    void reloadSessions();
  }
}

// ── initial load ──────────────────────────────────────────────────────

async function loadInitial(): Promise<void> {
  try {
    const r = await fetch('/health');
    state.serverOk = r.ok;
  } catch {
    state.serverOk = false;
  }
  updateStatus();
  try {
    state.providers = await getProviders();
    state.models = await getModels();
    const first = state.providers.find((p) => p.available) ?? state.providers[0];
    if (first) {
      state.provider = first.id;
      const m = state.models.find((x) => x.providerId === first.id);
      if (m) state.model = m.modelId;
    }
    renderProviderPicker();
    renderModelPicker();
    updateProviderModelDisplay();
  } catch (e) {
    console.error('initial provider/model load failed', e);
  }
  await reloadSessions();
  renderMessages();
  updateProviderModelDisplay();
  updateTokensDisplay();
}

// ── public init ───────────────────────────────────────────────────────

export function init(): void {
  if (ready) return;
  ready = true;

  const sendBtn = $('chat-send');
  sendBtn?.addEventListener('click', () => void send());
  const input = $('chat-input') as HTMLTextAreaElement | null;
  input?.addEventListener('keydown', (e) => {
    const ke = e as KeyboardEvent;
    if (ke.key === 'Enter' && !ke.shiftKey) {
      ke.preventDefault();
      void send();
    }
  });
  $('new-session-btn')?.addEventListener('click', () => void newSession());

  // surface the chat API to the command palette (separate <script>)
  (window as unknown as { __orion_chat: unknown }).__orion_chat = {
    newSession: () => void newSession(),
    deleteCurrentSession: () => void deleteCurrentSession(),
    reloadSessions: () => void reloadSessions(),
  };

  // initial chrome paint
  updateStatus();
  updateProviderModelDisplay();
  updateTokensDisplay();
  renderMessages();

  // defer the network calls so the UI paints first
  void loadInitial();
}

// Auto-init in the browser. In SSR contexts (Astro server render of this
// module) document is undefined; the guard makes that a no-op.
if (typeof document !== 'undefined') {
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => init());
  } else {
    init();
  }
}
