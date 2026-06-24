import { e as createComponent, m as maybeRenderHead, s as spreadAttributes, g as addAttribute, k as renderSlot, r as renderTemplate, h as createAstro, l as renderComponent, n as renderHead, o as renderScript } from '../chunks/astro/server_BSYTUYV-.mjs';
import 'piccolore';
/* empty css                                 */
import 'clsx';
export { renderers } from '../renderers.mjs';

const $$Astro$f = createAstro();
const $$ = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$f, $$props, $$slots);
  Astro2.self = $$;
  const size = Astro2.props.size;
  const cls = Astro2.props.class;
  const name = Astro2.props.iconName;
  delete Astro2.props.size;
  delete Astro2.props.class;
  delete Astro2.props.iconName;
  const props = Object.assign({
    "xmlns": "http://www.w3.org/2000/svg",
    "stroke-width": 2,
    "width": size ?? 24,
    "height": size ?? 24,
    "stroke": "currentColor",
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
    "fill": "none",
    "viewBox": "0 0 24 24"
  }, Astro2.props);
  return renderTemplate`${maybeRenderHead()}<svg${spreadAttributes(props)}${addAttribute(["lucide", { [`lucide-${name}`]: name }, cls], "class:list")}> ${renderSlot($$result, $$slots["default"])} </svg>`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/node_modules/lucide-astro/dist/.Layout.astro", void 0);

const $$Astro$e = createAstro();
const $$Brain = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$e, $$props, $$slots);
  Astro2.self = $$Brain;
  return renderTemplate`${renderComponent($$result, "Layout", $$, { "iconName": "brain", ...Astro2.props }, { "default": ($$result2) => renderTemplate` ${maybeRenderHead()}<path d="M12 18V5"></path> <path d="M15 13a4.17 4.17 0 0 1-3-4 4.17 4.17 0 0 1-3 4"></path> <path d="M17.598 6.5A3 3 0 1 0 12 5a3 3 0 1 0-5.598 1.5"></path> <path d="M17.997 5.125a4 4 0 0 1 2.526 5.77"></path> <path d="M18 18a4 4 0 0 0 2-7.464"></path> <path d="M19.967 17.483A4 4 0 1 1 12 18a4 4 0 1 1-7.967-.517"></path> <path d="M6 18a4 4 0 0 1-2-7.464"></path> <path d="M6.003 5.125a4 4 0 0 0-2.526 5.77"></path> ` })}`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/node_modules/lucide-astro/dist/Brain.astro", void 0);

const $$Astro$d = createAstro();
const $$Circle = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$d, $$props, $$slots);
  Astro2.self = $$Circle;
  return renderTemplate`${renderComponent($$result, "Layout", $$, { "iconName": "circle", ...Astro2.props }, { "default": ($$result2) => renderTemplate` ${maybeRenderHead()}<circle cx="12" cy="12" r="10"></circle> ` })}`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/node_modules/lucide-astro/dist/Circle.astro", void 0);

const $$Astro$c = createAstro();
const $$Globe = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$c, $$props, $$slots);
  Astro2.self = $$Globe;
  return renderTemplate`${renderComponent($$result, "Layout", $$, { "iconName": "globe", ...Astro2.props }, { "default": ($$result2) => renderTemplate` ${maybeRenderHead()}<circle cx="12" cy="12" r="10"></circle> <path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"></path> <path d="M2 12h20"></path> ` })}`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/node_modules/lucide-astro/dist/Globe.astro", void 0);

const $$Astro$b = createAstro();
const $$Keyboard = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$b, $$props, $$slots);
  Astro2.self = $$Keyboard;
  return renderTemplate`${renderComponent($$result, "Layout", $$, { "iconName": "keyboard", ...Astro2.props }, { "default": ($$result2) => renderTemplate` ${maybeRenderHead()}<path d="M10 8h.01"></path> <path d="M12 12h.01"></path> <path d="M14 8h.01"></path> <path d="M16 12h.01"></path> <path d="M18 8h.01"></path> <path d="M6 8h.01"></path> <path d="M7 16h10"></path> <path d="M8 12h.01"></path> <rect width="20" height="16" x="2" y="4" rx="2"></rect> ` })}`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/node_modules/lucide-astro/dist/Keyboard.astro", void 0);

const $$Astro$a = createAstro();
const $$Lock = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$a, $$props, $$slots);
  Astro2.self = $$Lock;
  return renderTemplate`${renderComponent($$result, "Layout", $$, { "iconName": "lock", ...Astro2.props }, { "default": ($$result2) => renderTemplate` ${maybeRenderHead()}<rect width="18" height="11" x="3" y="11" rx="2" ry="2"></rect> <path d="M7 11V7a5 5 0 0 1 10 0v4"></path> ` })}`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/node_modules/lucide-astro/dist/Lock.astro", void 0);

const $$Astro$9 = createAstro();
const $$MessageSquare = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$9, $$props, $$slots);
  Astro2.self = $$MessageSquare;
  return renderTemplate`${renderComponent($$result, "Layout", $$, { "iconName": "message-square", ...Astro2.props }, { "default": ($$result2) => renderTemplate` ${maybeRenderHead()}<path d="M22 17a2 2 0 0 1-2 2H6.828a2 2 0 0 0-1.414.586l-2.202 2.202A.71.71 0 0 1 2 21.286V5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2z"></path> ` })}`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/node_modules/lucide-astro/dist/MessageSquare.astro", void 0);

const $$Astro$8 = createAstro();
const $$Palette = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$8, $$props, $$slots);
  Astro2.self = $$Palette;
  return renderTemplate`${renderComponent($$result, "Layout", $$, { "iconName": "palette", ...Astro2.props }, { "default": ($$result2) => renderTemplate` ${maybeRenderHead()}<path d="M12 22a1 1 0 0 1 0-20 10 9 0 0 1 10 9 5 5 0 0 1-5 5h-2.25a1.75 1.75 0 0 0-1.4 2.8l.3.4a1.75 1.75 0 0 1-1.4 2.8z"></path> <circle cx="13.5" cy="6.5" r=".5" fill="currentColor"></circle> <circle cx="17.5" cy="10.5" r=".5" fill="currentColor"></circle> <circle cx="6.5" cy="12.5" r=".5" fill="currentColor"></circle> <circle cx="8.5" cy="7.5" r=".5" fill="currentColor"></circle> ` })}`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/node_modules/lucide-astro/dist/Palette.astro", void 0);

const $$Astro$7 = createAstro();
const $$Plug = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$7, $$props, $$slots);
  Astro2.self = $$Plug;
  return renderTemplate`${renderComponent($$result, "Layout", $$, { "iconName": "plug", ...Astro2.props }, { "default": ($$result2) => renderTemplate` ${maybeRenderHead()}<path d="M12 22v-5"></path> <path d="M9 8V2"></path> <path d="M15 8V2"></path> <path d="M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8Z"></path> ` })}`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/node_modules/lucide-astro/dist/Plug.astro", void 0);

const $$Astro$6 = createAstro();
const $$Server = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$6, $$props, $$slots);
  Astro2.self = $$Server;
  return renderTemplate`${renderComponent($$result, "Layout", $$, { "iconName": "server", ...Astro2.props }, { "default": ($$result2) => renderTemplate` ${maybeRenderHead()}<rect width="20" height="8" x="2" y="2" rx="2" ry="2"></rect> <rect width="20" height="8" x="2" y="14" rx="2" ry="2"></rect> <line x1="6" x2="6.01" y1="6" y2="6"></line> <line x1="6" x2="6.01" y1="18" y2="18"></line> ` })}`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/node_modules/lucide-astro/dist/Server.astro", void 0);

const $$Astro$5 = createAstro();
const $$Settings = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$5, $$props, $$slots);
  Astro2.self = $$Settings;
  return renderTemplate`${renderComponent($$result, "Layout", $$, { "iconName": "settings", ...Astro2.props }, { "default": ($$result2) => renderTemplate` ${maybeRenderHead()}<path d="M9.671 4.136a2.34 2.34 0 0 1 4.659 0 2.34 2.34 0 0 0 3.319 1.915 2.34 2.34 0 0 1 2.33 4.033 2.34 2.34 0 0 0 0 3.831 2.34 2.34 0 0 1-2.33 4.033 2.34 2.34 0 0 0-3.319 1.915 2.34 2.34 0 0 1-4.659 0 2.34 2.34 0 0 0-3.32-1.915 2.34 2.34 0 0 1-2.33-4.033 2.34 2.34 0 0 0 0-3.831A2.34 2.34 0 0 1 6.35 6.051a2.34 2.34 0 0 0 3.319-1.915"></path> <circle cx="12" cy="12" r="3"></circle> ` })}`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/node_modules/lucide-astro/dist/Settings.astro", void 0);

const $$Astro$4 = createAstro();
const $$MemoryPanel = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$4, $$props, $$slots);
  Astro2.self = $$MemoryPanel;
  return renderTemplate`${maybeRenderHead()}<div class="settings"> <h2>Memory</h2> <div class="group"> <h3>Project summary</h3> <p class="muted" style="margin-top: 0; font-size: 13px;">
ORION can summarize a session with the active LLM and store the digest
      as project-level context. Summaries are auto-injected into future
      conversations in the same working directory.
</p> <p class="muted" style="font-size: 12px; margin-bottom: 0;">
This panel is a read-only preview. Trigger summarization through the
<code>POST /api/sessions/:id/summarize</code> endpoint or the
<code>orion memory summarize</code> CLI command.
</p> </div> <div class="group"> <h3>Token budget (token-god)</h3> <p class="muted" style="margin-top: 0; font-size: 13px;">
Token optimization is wired to the token-god MCP server. Auto-compression
      triggers when a session exceeds 75% of its context window.
</p> <p class="muted" style="font-size: 12px; margin-bottom: 0;">
Configure thresholds under
<code>Settings &rarr; Token budget / session</code> or in
<code>~/.orion/settings.json</code>.
</p> </div> </div>`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/src/components/MemoryPanel.astro", void 0);

const $$Astro$3 = createAstro();
const $$ServersPanel = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$3, $$props, $$slots);
  Astro2.self = $$ServersPanel;
  return renderTemplate`${maybeRenderHead()}<div class="servers"> <h2>MCP Servers</h2> <p class="muted" style="margin-top: 0;">
Model Context Protocol servers connected to ORION core.
</p> <div class="msg system">
Server list is read-only in this view. Inspect the live registry at
<code>GET /api/mcp/servers</code> or add a new server with
<code>orion mcp add &lt;id&gt; --transport stdio</code>.
</div> <div class="msg system" style="margin-top: 8px;">
Statuses reflect the same three values the core reports:
<code>active</code>, <code>configured</code>, and <code>idle</code>.
    Transport kinds supported: <code>stdio</code>, <code>http</code>, <code>sse</code>.
</div> </div>`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/src/components/ServersPanel.astro", void 0);

const $$Astro$2 = createAstro();
const $$SettingsPanel = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$2, $$props, $$slots);
  Astro2.self = $$SettingsPanel;
  const sections = [
    { id: "general", label: "General", icon: $$Settings, group: "General", blurb: "behavior, permissions, default shell" },
    { id: "providers", label: "Providers", icon: $$Plug, group: "General", blurb: "connected LLM providers and credentials" },
    { id: "language", label: "Language", icon: $$Globe, group: "General", blurb: "interface and agent response language" },
    { id: "appearance", label: "Appearance", icon: $$Palette, group: "Interface", blurb: "themes, color scheme, UI/code fonts" },
    { id: "shortcuts", label: "Shortcuts", icon: $$Keyboard, group: "Interface", blurb: "keyboard binding reference" },
    { id: "servers", label: "Servers", icon: $$Server, group: "System", blurb: "MCP server registry" },
    { id: "memory", label: "Memory", icon: $$Brain, group: "System", blurb: "project memory and token budget" },
    { id: "permissions", label: "Permissions", icon: $$Lock, group: "System", blurb: "per-action allow/ask/block policy" }
  ];
  const groups = ["General", "Interface", "System"];
  return renderTemplate`${maybeRenderHead()}<div class="settings-shell" data-astro-cid-da3ypr3c> <nav class="settings-nav" data-astro-cid-da3ypr3c> <div class="settings-nav-top" data-astro-cid-da3ypr3c> <div data-astro-cid-da3ypr3c> <div class="settings-nav-title" data-astro-cid-da3ypr3c>Settings</div> <div class="settings-nav-ver" data-astro-cid-da3ypr3c>Orion v2.0</div> </div> </div> ${groups.map((g) => renderTemplate`<div data-astro-cid-da3ypr3c> <div class="settings-nav-label" data-astro-cid-da3ypr3c>${g}</div> ${sections.filter((s) => s.group === g).map((s) => {
    const Icon = s.icon;
    return renderTemplate`<div class="settings-ni"${addAttribute(s.id, "data-section")} data-astro-cid-da3ypr3c> <i data-astro-cid-da3ypr3c>${renderComponent($$result, "Icon", Icon, { "size": 16, "data-astro-cid-da3ypr3c": true })}</i> ${s.label} </div>`;
  })} </div>`)} </nav> <div class="settings-content" data-astro-cid-da3ypr3c> <div class="sh" data-astro-cid-da3ypr3c>Settings</div> <div class="sd" data-astro-cid-da3ypr3c>
Read-only preview. To modify settings, use the ORION core API
      (<code data-astro-cid-da3ypr3c>PUT /api/settings</code>) or edit <code data-astro-cid-da3ypr3c>~/.orion/settings.json</code>
directly — changes are picked up on the next request.
</div> <div class="group" data-astro-cid-da3ypr3c> <h3 data-astro-cid-da3ypr3c>Available sections</h3> <ul class="settings-overview" data-astro-cid-da3ypr3c> ${sections.map((s) => renderTemplate`<li data-astro-cid-da3ypr3c> <strong data-astro-cid-da3ypr3c>${s.label}</strong> <span class="muted" data-astro-cid-da3ypr3c>— ${s.blurb}</span> </li>`)} </ul> </div> <div class="group" data-astro-cid-da3ypr3c> <h3 data-astro-cid-da3ypr3c>Live editing</h3> <p class="muted" style="font-size: 13px; margin: 0;" data-astro-cid-da3ypr3c>
Interactive toggles, theme pickers, custom theme editor, and the
        MCP server form have been moved to the core API surface. The UI
        layer in this build is a read-only shell — server-side state stays
        the single source of truth and the browser stays free of editor
        state machinery.
</p> </div> </div> </div> `;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/src/components/SettingsPanel.astro", void 0);

const CORE = "/api";
async function getProviders() {
  const res = await fetch(`${CORE}/providers`);
  if (!res.ok) return [];
  return res.json();
}
async function getModels(opts = {}) {
  const q = new URLSearchParams();
  if (opts.free) q.set("free", "true");
  const res = await fetch(`${CORE}/models?${q.toString()}`);
  if (!res.ok) return [];
  return res.json();
}
async function listSessions() {
  const res = await fetch(`${CORE}/sessions`);
  if (!res.ok) return [];
  return res.json();
}
async function createSession(input) {
  const res = await fetch(`${CORE}/sessions`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(input)
  });
  if (!res.ok) throw new Error(`createSession failed: ${res.status}`);
  return res.json();
}
async function deleteSession(id) {
  await fetch(`${CORE}/sessions/${id}`, { method: "DELETE" });
}
async function* streamChat(req) {
  const res = await fetch(`${CORE}/chat`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(req)
  });
  if (!res.ok || !res.body) {
    throw new Error(`chat failed: ${res.status}`);
  }
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let idx;
    while ((idx = buffer.indexOf("\n")) >= 0) {
      const line = buffer.slice(0, idx).trimEnd();
      buffer = buffer.slice(idx + 1);
      if (!line.startsWith("data:")) continue;
      const data = line.slice(5).trim();
      if (data === "[DONE]") return;
      if (data.length === 0) continue;
      yield data;
    }
  }
}

const state = {
  messages: [],
  provider: "",
  model: "",
  sessionId: typeof crypto !== "undefined" && "randomUUID" in crypto ? crypto.randomUUID() : Math.random().toString(36).slice(2),
  busy: false,
  sessions: [],
  providers: [],
  models: [],
  tokensUsed: 0,
  serverOk: false
};
let ready = false;
function $(id) {
  return document.getElementById(id);
}
function el(tag, attrs = {}, text) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  if (text !== void 0) node.textContent = text;
  return node;
}
function setView(view) {
  document.documentElement.setAttribute("data-view", view);
  document.querySelectorAll("[data-view-panel]").forEach((p) => {
    if (p.getAttribute("data-view-panel") === view) p.setAttribute("data-active", "");
    else p.removeAttribute("data-active");
  });
  document.querySelectorAll("nav.sidebar button[data-view]").forEach((b) => {
    if (b.getAttribute("data-view") === view) b.classList.add("active");
    else b.classList.remove("active");
  });
}
function updateProviderModelDisplay() {
  setText("crumbs-info", `${state.provider}/${state.model || "—"}`);
  setText("provider-name", state.provider || "—");
  setText("model-name", state.model || "—");
  setText("meta-provider", state.provider || "—");
  setText("meta-model", state.model || "—");
  setText("session-id", state.sessionId.slice(0, 8));
  setText("meta-session", state.sessionId.slice(0, 8));
}
function updateStatus() {
  const status = $("server-status");
  if (status) status.setAttribute("data-online", state.serverOk ? "true" : "false");
  setText("server-status-text", state.serverOk ? "online" : "offline");
}
function updateTokensDisplay() {
  setText("tokens-count", Math.round(state.tokensUsed).toLocaleString());
  const bar = $("tokens-bar");
  if (bar) bar.style.width = `${Math.min(100, state.tokensUsed / 2e5 * 100)}%`;
}
function setText(id, text) {
  const node = $(id);
  if (node) node.textContent = text;
}
function renderProviderPicker() {
  const picker = $("provider-picker");
  if (!picker) return;
  picker.innerHTML = "";
  for (const p of state.providers) {
    const opt = el("option", { value: p.id });
    opt.textContent = `${p.name} ${p.available ? "· online" : "· offline"}`;
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
function renderModelPicker() {
  const picker = $("model-picker");
  if (!picker) return;
  picker.innerHTML = "";
  const filtered = state.models.filter((m) => m.providerId === state.provider);
  if (filtered.length === 0) {
    const opt = el("option", { value: "" }, "no models synced");
    picker.appendChild(opt);
    picker.disabled = true;
    return;
  }
  picker.disabled = false;
  for (const m of filtered) {
    const opt = el("option", { value: m.modelId });
    opt.textContent = `${m.displayName || m.modelId} ${m.isFree ? "·free" : ""}`;
    picker.appendChild(opt);
  }
  picker.value = state.model;
  picker.onchange = () => {
    state.model = picker.value;
    updateProviderModelDisplay();
  };
}
function renderSessionsList() {
  const list = $("sessions-list");
  if (!list) return;
  setText("sessions-count", String(state.sessions.length));
  list.innerHTML = "";
  if (state.sessions.length === 0) {
    list.appendChild(
      el("div", { class: "sidebar-empty" }, "No sessions yet. Send a message to start one.")
    );
    return;
  }
  for (const s of state.sessions.slice(0, 10)) {
    const btn = el("button", {
      class: s.id === state.sessionId ? "active" : "",
      title: `${s.provider}/${s.model} · ${s.message_count} msgs`
    });
    const label = el("span", { class: "session-label" }, s.title);
    const pill = el("span", { class: "pill" }, String(s.message_count));
    btn.appendChild(label);
    btn.appendChild(pill);
    btn.addEventListener("click", () => selectSession(s.id));
    list.appendChild(btn);
  }
}
function selectSession(id) {
  state.sessionId = id;
  state.messages = [];
  renderMessages();
  renderSessionsList();
  updateProviderModelDisplay();
  setView("chat");
}
async function newSession() {
  try {
    const s = await createSession({ title: "New session", provider: state.provider, model: state.model });
    state.sessionId = s.id;
    state.messages = [];
    await reloadSessions();
    renderMessages();
    updateProviderModelDisplay();
    setView("chat");
  } catch (e) {
    console.error("newSession failed", e);
  }
}
async function deleteCurrentSession() {
  if (!state.sessionId) return;
  try {
    await deleteSession(state.sessionId);
  } catch (e) {
    console.error("deleteSession failed", e);
  }
  state.sessionId = typeof crypto !== "undefined" && "randomUUID" in crypto ? crypto.randomUUID() : Math.random().toString(36).slice(2);
  state.messages = [];
  await reloadSessions();
  renderMessages();
  updateProviderModelDisplay();
}
async function reloadSessions() {
  try {
    state.sessions = await listSessions();
  } catch {
    state.sessions = [];
  }
  renderSessionsList();
}
function renderMessages() {
  const container = $("messages");
  if (!container) return;
  container.innerHTML = "";
  if (state.messages.length === 0) {
    const welcome = el("div", { class: "msg system" });
    welcome.appendChild(el("div", { class: "role" }, "ORION"));
    welcome.appendChild(document.createTextNode(" Connected to "));
    welcome.appendChild(el("b", {}, state.provider || "—"));
    welcome.appendChild(document.createTextNode(" · "));
    welcome.appendChild(el("b", {}, state.model || "—"));
    welcome.appendChild(document.createTextNode(". Send a message to begin."));
    container.appendChild(welcome);
  } else {
    for (const m of state.messages) {
      const msg = el("div", {
        class: `msg ${m.role}${m.streaming ? " cursor-blink" : ""}`
      });
      msg.appendChild(el("div", { class: "role" }, m.role));
      const text = m.content || (m.streaming ? "…" : "");
      msg.appendChild(document.createTextNode(text));
      container.appendChild(msg);
    }
  }
  requestAnimationFrame(() => {
    container.scrollTo({ top: container.scrollHeight, behavior: "smooth" });
  });
}
async function send() {
  const input = $("chat-input");
  if (!input) return;
  const text = input.value.trim();
  if (!text || state.busy) return;
  input.value = "";
  state.busy = true;
  const sendBtn = $("chat-send");
  if (sendBtn) sendBtn.disabled = true;
  const userMsg = {
    id: crypto.randomUUID(),
    role: "user",
    content: text,
    createdAt: Date.now()
  };
  const assistantId = crypto.randomUUID();
  state.messages.push(userMsg);
  state.messages.push({
    id: assistantId,
    role: "assistant",
    content: "",
    streaming: true,
    createdAt: Date.now()
  });
  renderMessages();
  try {
    const history = state.messages.filter((m2) => !m2.streaming && m2.id !== assistantId).map((m2) => ({ role: m2.role, content: m2.content }));
    const gen = streamChat({
      provider: state.provider,
      model: state.model,
      message: text,
      sessionId: state.sessionId,
      history
    });
    let acc = "";
    for await (const tok of gen) {
      acc += tok;
      state.tokensUsed += tok.length / 4;
      const m2 = state.messages.find((x) => x.id === assistantId);
      if (m2) m2.content = acc;
      updateTokensDisplay();
      renderMessages();
    }
    const m = state.messages.find((x) => x.id === assistantId);
    if (m) m.streaming = false;
    renderMessages();
  } catch (e) {
    const m = state.messages.find((x) => x.id === assistantId);
    if (m) {
      m.content = `[error] ${e.message}`;
      m.streaming = false;
    }
    renderMessages();
  } finally {
    state.busy = false;
    if (sendBtn) sendBtn.disabled = false;
    void reloadSessions();
  }
}
async function loadInitial() {
  try {
    const r = await fetch("/health");
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
    console.error("initial provider/model load failed", e);
  }
  await reloadSessions();
  renderMessages();
  updateProviderModelDisplay();
  updateTokensDisplay();
}
function init() {
  if (ready) return;
  ready = true;
  const sendBtn = $("chat-send");
  sendBtn?.addEventListener("click", () => void send());
  const input = $("chat-input");
  input?.addEventListener("keydown", (e) => {
    const ke = e;
    if (ke.key === "Enter" && !ke.shiftKey) {
      ke.preventDefault();
      void send();
    }
  });
  $("new-session-btn")?.addEventListener("click", () => void newSession());
  window.__orion_chat = {
    newSession: () => void newSession(),
    deleteCurrentSession: () => void deleteCurrentSession(),
    reloadSessions: () => void reloadSessions()
  };
  updateStatus();
  updateProviderModelDisplay();
  updateTokensDisplay();
  renderMessages();
  void loadInitial();
}
if (typeof document !== "undefined") {
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", () => init());
  } else {
    init();
  }
}

const $$Astro$1 = createAstro();
const $$App = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro$1, $$props, $$slots);
  Astro2.self = $$App;
  const { title = "ORION" } = Astro2.props;
  init();
  return renderTemplate`<html lang="en" data-view="chat" data-astro-cid-mnwxwo2t> <head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>${title}</title>${renderHead()}</head> <body data-astro-cid-mnwxwo2t> <div class="app" data-astro-cid-mnwxwo2t> <header class="topbar" data-astro-cid-mnwxwo2t> <span class="brand" data-astro-cid-mnwxwo2t>⬢ ORION</span> <span class="crumbs" data-astro-cid-mnwxwo2t>chat · <span id="crumbs-info" data-astro-cid-mnwxwo2t>—</span></span> <span class="spacer" data-astro-cid-mnwxwo2t></span> <span class="status" id="server-status" data-online="false" data-astro-cid-mnwxwo2t> ${renderComponent($$result, "Circle", $$Circle, { "size": 12, "data-astro-cid-mnwxwo2t": true })} core <span id="server-status-text" data-astro-cid-mnwxwo2t>offline</span> </span> </header> <nav class="sidebar" data-astro-cid-mnwxwo2t> <h3 data-astro-cid-mnwxwo2t>Workspace</h3> <button data-view="chat" class="active" data-astro-cid-mnwxwo2t>${renderComponent($$result, "MessageSquare", $$MessageSquare, { "size": 16, "data-astro-cid-mnwxwo2t": true })} Chat</button> <button data-view="memory" data-astro-cid-mnwxwo2t>${renderComponent($$result, "Brain", $$Brain, { "size": 16, "data-astro-cid-mnwxwo2t": true })} Memory</button> <h3 data-astro-cid-mnwxwo2t>Sessions (<span id="sessions-count" data-astro-cid-mnwxwo2t>0</span>)</h3> <div id="sessions-list" data-astro-cid-mnwxwo2t> <div class="sidebar-empty" data-astro-cid-mnwxwo2t>No sessions yet. Send a message to start one.</div> </div> <h3 data-astro-cid-mnwxwo2t>System</h3> <button data-view="servers" data-astro-cid-mnwxwo2t>${renderComponent($$result, "Server", $$Server, { "size": 16, "data-astro-cid-mnwxwo2t": true })} Servers</button> <button data-view="settings" data-astro-cid-mnwxwo2t>${renderComponent($$result, "SettingsIcon", $$Settings, { "size": 16, "data-astro-cid-mnwxwo2t": true })} Settings</button> <h3 data-astro-cid-mnwxwo2t>Provider</h3> <select id="provider-picker" class="sel" style="width: 100%; margin-bottom: 8px;" data-astro-cid-mnwxwo2t> <option data-astro-cid-mnwxwo2t>Loading…</option> </select> <select id="model-picker" class="sel" style="width: 100%;" data-astro-cid-mnwxwo2t> <option data-astro-cid-mnwxwo2t>—</option> </select> <button id="new-session-btn" class="new-session-btn" data-astro-cid-mnwxwo2t>+ New session</button> <div class="palette-hint" data-astro-cid-mnwxwo2t>Ctrl+K command palette</div> </nav> <main class="main" data-astro-cid-mnwxwo2t> <div class="view-panel" data-view-panel="chat" data-active data-astro-cid-mnwxwo2t> ${renderSlot($$result, $$slots["default"])} </div> <div class="view-panel" data-view-panel="memory" data-astro-cid-mnwxwo2t> ${renderComponent($$result, "MemoryPanel", $$MemoryPanel, { "data-astro-cid-mnwxwo2t": true })} </div> <div class="view-panel" data-view-panel="servers" data-astro-cid-mnwxwo2t> ${renderComponent($$result, "ServersPanel", $$ServersPanel, { "data-astro-cid-mnwxwo2t": true })} </div> <div class="view-panel" data-view-panel="settings" data-astro-cid-mnwxwo2t> ${renderComponent($$result, "SettingsPanel", $$SettingsPanel, { "data-astro-cid-mnwxwo2t": true })} </div> </main> <footer class="statusbar" data-astro-cid-mnwxwo2t> <span data-astro-cid-mnwxwo2t>tokens: <span id="tokens-count" data-astro-cid-mnwxwo2t>0</span></span> <div class="meter" data-astro-cid-mnwxwo2t><div class="bar" id="tokens-bar" style="width: 0%;" data-astro-cid-mnwxwo2t></div></div> <span data-astro-cid-mnwxwo2t>session: <span id="session-id" data-astro-cid-mnwxwo2t>—</span></span> <span style="margin-left: auto; opacity: 0.7;" data-astro-cid-mnwxwo2t>Ctrl+K · palette</span> </footer> </div> <div id="palette" class="palette-backdrop" hidden data-astro-cid-mnwxwo2t> <div class="palette-panel" id="palette-panel" data-astro-cid-mnwxwo2t> <input id="palette-input" type="text" placeholder="Type a command…" autocomplete="off" spellcheck="false" data-astro-cid-mnwxwo2t> <div id="palette-results" data-astro-cid-mnwxwo2t></div> </div> </div> ${renderScript($$result, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/src/layouts/App.astro?astro&type=script&index=0&lang.ts")}  </body> </html>`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/src/layouts/App.astro", void 0);

const $$Astro = createAstro();
const $$ChatIsland = createComponent(($$result, $$props, $$slots) => {
  const Astro2 = $$result.createAstro($$Astro, $$props, $$slots);
  Astro2.self = $$ChatIsland;
  return renderTemplate`${maybeRenderHead()}<div class="chat-island"> <div id="messages" class="messages"> <div class="msg system" id="welcome-msg"> <div class="role">ORION</div> <span id="welcome-text">
Connected to <b id="provider-name">—</b> · <b id="model-name">—</b>. Send a message to begin.
</span> </div> </div> <div class="composer"> <div class="row"> <textarea id="chat-input" placeholder="Ask ORION anything. Enter to send, Shift+Enter for newline." rows="2" autocomplete="off" spellcheck="false"></textarea> <button id="chat-send" class="send">Send</button> </div> <div class="meta"> <span>provider: <span id="meta-provider">—</span></span> <span>model: <span id="meta-model">—</span></span> <span style="margin-left: auto;">session: <span id="meta-session">—</span></span> </div> </div> </div>`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/src/components/ChatIsland.astro", void 0);

const $$Index = createComponent(($$result, $$props, $$slots) => {
  return renderTemplate`${renderComponent($$result, "App", $$App, { "title": "ORION" }, { "default": ($$result2) => renderTemplate` ${renderComponent($$result2, "ChatIsland", $$ChatIsland, {})} ` })}`;
}, "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/src/pages/index.astro", void 0);

const $$file = "/home/jcperez/Desktop/workspace/orion/apps/orion-ui/src/pages/index.astro";
const $$url = "";

const _page = /*#__PURE__*/Object.freeze(/*#__PURE__*/Object.defineProperty({
	__proto__: null,
	default: $$Index,
	file: $$file,
	url: $$url
}, Symbol.toStringTag, { value: 'Module' }));

const page = () => _page;

export { page };
