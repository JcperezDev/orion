//! Minimal browser-based web UI for ORION.
//!
//! A single-page HTML chat interface served at `/`. It uses Server-Sent Events
//! (SSE) to stream chat responses from `/api/chat`.

use axum::{response::Html, Json};
use serde_json::{json, Value};

/// `GET /` — returns the Web UI HTML.
pub async fn index() -> Html<&'static str> {
    Html(WEB_UI_HTML)
}

/// `GET /api/health` — JSON health check.
pub async fn health_json() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "name": "orion-server",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// `GET /api/info` — server metadata.
pub async fn info() -> Json<Value> {
    Json(json!({
        "name": "orion-server",
        "version": env!("CARGO_PKG_VERSION"),
        "agents": ["build", "plan", "explore", "scout", "general"],
        "tools": [
            "read", "write", "edit", "bash", "grep", "glob",
            "webfetch", "websearch", "question", "todowrite",
            "apply_patch", "lsp", "pty", "task"
        ],
        "endpoints": {
            "chat": "POST /api/chat",
            "stream": "GET /api/stream/:session_id",
            "sessions": "GET /api/sessions",
            "providers": "GET /api/providers",
            "models": "GET /api/models",
            "acp": "POST /acp (stdio via separate binary)"
        }
    }))
}

/// Minimal, dependency-free HTML/JS for the Web UI.
///
/// Uses SSE to stream chat from `/api/chat`. The interface mirrors a basic
/// chat with model selector, message list, and input box.
const WEB_UI_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>ORION Web</title>
<style>
:root {
    --bg: #0e0f12;
    --fg: #e6e6e6;
    --muted: #8b949e;
    --accent: #58a6ff;
    --accent-hover: #79b8ff;
    --border: #30363d;
    --code-bg: #161b22;
    --user: #1f6feb;
    --assistant: #238636;
    --system: #8b949e;
    --error: #f85149;
}
* { box-sizing: border-box; }
body {
    margin: 0;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Helvetica Neue", Arial, sans-serif;
    background: var(--bg);
    color: var(--fg);
    height: 100vh;
    display: flex;
    flex-direction: column;
}
header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    background: var(--code-bg);
}
header h1 {
    font-size: 18px;
    margin: 0;
    color: var(--accent);
}
header .subtitle {
    color: var(--muted);
    font-size: 12px;
}
header .controls {
    margin-left: auto;
    display: flex;
    gap: 8px;
}
select, button, input, textarea {
    background: var(--bg);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 10px;
    font: inherit;
}
button {
    cursor: pointer;
    background: var(--accent);
    color: #fff;
    border: none;
    font-weight: 600;
}
button:hover { background: var(--accent-hover); }
button:disabled { opacity: 0.5; cursor: not-allowed; }
main {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
}
.msg {
    max-width: 80%;
    padding: 10px 14px;
    border-radius: 12px;
    word-wrap: break-word;
    white-space: pre-wrap;
    line-height: 1.5;
}
.msg.user {
    align-self: flex-end;
    background: var(--user);
    color: #fff;
}
.msg.assistant {
    align-self: flex-start;
    background: var(--code-bg);
    border: 1px solid var(--border);
}
.msg.system {
    align-self: center;
    color: var(--muted);
    font-size: 12px;
}
.msg.error {
    align-self: center;
    background: var(--error);
    color: #fff;
}
.msg .role {
    font-size: 11px;
    text-transform: uppercase;
    opacity: 0.7;
    margin-bottom: 4px;
}
footer {
    border-top: 1px solid var(--border);
    padding: 12px 16px;
    display: flex;
    gap: 8px;
    background: var(--code-bg);
}
footer textarea {
    flex: 1;
    resize: vertical;
    min-height: 40px;
    max-height: 200px;
    font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, monospace;
}
.spinner {
    display: inline-block;
    width: 12px;
    height: 12px;
    border: 2px solid var(--muted);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
}
@keyframes spin { to { transform: rotate(360deg); } }
</style>
</head>
<body>
<header>
    <h1>ORION</h1>
    <span class="subtitle">multi-provider coding agent</span>
    <div class="controls">
        <select id="model-select" title="Model">
            <option value="">loading…</option>
        </select>
        <select id="agent-select" title="Agent">
            <option value="build">build</option>
            <option value="plan">plan</option>
            <option value="explore">explore</option>
            <option value="scout">scout</option>
            <option value="general">general</option>
        </select>
        <button id="clear-btn" title="Clear chat">Clear</button>
    </div>
</header>
<main id="messages">
    <div class="msg system"><div class="role">orion</div>Welcome. Pick a model, type a message, and press ⏎ or Send.</div>
</main>
<footer>
    <textarea id="input" placeholder="Ask ORION anything…" rows="1"></textarea>
    <button id="send-btn">Send</button>
</footer>
<script>
const $ = (id) => document.getElementById(id);
const messagesEl = $("messages");
const inputEl = $("input");
const sendBtn = $("send-btn");
const modelSelect = $("model-select");
const agentSelect = $("agent-select");
const clearBtn = $("clear-btn");

let sessionId = null;
let busy = false;

function addMessage(role, text) {
    const div = document.createElement("div");
    div.className = "msg " + role;
    const roleEl = document.createElement("div");
    roleEl.className = "role";
    roleEl.textContent = role;
    const contentEl = document.createElement("div");
    contentEl.textContent = text;
    div.appendChild(roleEl);
    div.appendChild(contentEl);
    messagesEl.appendChild(div);
    messagesEl.scrollTop = messagesEl.scrollHeight;
    return contentEl;
}

async function loadModels() {
    try {
        const r = await fetch("/api/models");
        const data = await r.json();
        const models = data.models || [];
        modelSelect.innerHTML = "";
        if (models.length === 0) {
            const opt = document.createElement("option");
            opt.value = "";
            opt.textContent = "(no models — run `orion login <provider>`)";
            modelSelect.appendChild(opt);
            return;
        }
        for (const m of models) {
            const opt = document.createElement("option");
            opt.value = m.full_id || `${m.provider_id}:${m.model_id}`;
            opt.textContent = m.display_name || opt.value;
            modelSelect.appendChild(opt);
        }
    } catch (e) {
        console.error("loadModels:", e);
    }
}

async function ensureSession() {
    if (sessionId) return sessionId;
    const r = await fetch("/api/sessions", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ title: "web session" }),
    });
    const data = await r.json();
    sessionId = data.id;
    return sessionId;
}

async function send() {
    if (busy) return;
    const text = inputEl.value.trim();
    if (!text) return;
    inputEl.value = "";
    busy = true;
    sendBtn.disabled = true;
    sendBtn.innerHTML = '<span class="spinner"></span>';

    addMessage("user", text);
    const assistantEl = addMessage("assistant", "");

    try {
        const sid = await ensureSession();
        const model = modelSelect.value;
        const body = {
            session_id: sid,
            model: model || undefined,
            message: text,
            agent: agentSelect.value || "build",
        };
        const r = await fetch("/api/chat", {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(body),
        });
        if (!r.ok || !r.body) {
            const t = await r.text();
            assistantEl.textContent = "Error: " + (t || r.statusText);
            return;
        }
        const reader = r.body.getReader();
        const decoder = new TextDecoder();
        let acc = "";
        while (true) {
            const { value, done } = await reader.read();
            if (done) break;
            acc += decoder.decode(value, { stream: true });
            assistantEl.textContent = acc;
            messagesEl.scrollTop = messagesEl.scrollHeight;
        }
    } catch (e) {
        assistantEl.textContent = "Error: " + e.message;
    } finally {
        busy = false;
        sendBtn.disabled = false;
        sendBtn.textContent = "Send";
    }
}

sendBtn.addEventListener("click", send);
inputEl.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        send();
    }
});
inputEl.addEventListener("input", () => {
    inputEl.style.height = "auto";
    inputEl.style.height = Math.min(inputEl.scrollHeight, 200) + "px";
});
clearBtn.addEventListener("click", () => {
    messagesEl.innerHTML = "";
    addMessage("system", "Cleared.");
});
loadModels();
</script>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_is_well_formed() {
        assert!(WEB_UI_HTML.starts_with("<!DOCTYPE html>"));
        assert!(WEB_UI_HTML.trim_end().ends_with("</html>"));
    }

    #[test]
    fn html_includes_branding() {
        assert!(WEB_UI_HTML.contains("ORION"));
        assert!(WEB_UI_HTML.contains("multi-provider"));
    }

    #[test]
    fn html_includes_streaming_logic() {
        assert!(WEB_UI_HTML.contains("fetch(\"/api/chat\""));
        assert!(WEB_UI_HTML.contains("getReader"));
    }

    #[test]
    fn html_includes_model_selector() {
        assert!(WEB_UI_HTML.contains("model-select"));
        assert!(WEB_UI_HTML.contains("/api/models"));
    }

    #[tokio::test]
    async fn index_returns_html() {
        let resp = index().await;
        assert!(resp.0.contains("ORION"));
    }

    #[tokio::test]
    async fn info_lists_agents() {
        let json = info().await;
        let agents = json.0["agents"].as_array().unwrap();
        assert!(agents.contains(&serde_json::Value::String("build".into())));
        assert!(agents.contains(&serde_json::Value::String("plan".into())));
    }
}
