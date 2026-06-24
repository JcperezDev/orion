export type Role = 'system' | 'user' | 'assistant';

export interface ChatMessage {
  id: string;
  role: Role;
  content: string;
  streaming?: boolean;
  createdAt: number;
}

export interface ProviderInfo {
  id: string;
  name: string;
  kind: string;
  enabled: boolean;
  available: boolean;
  baseUrl?: string | null;
  models: ModelInfo[];
}

export interface ModelInfo {
  fullId: string;
  providerId: string;
  modelId: string;
  displayName: string;
  contextWindow?: number | null;
  inputPrice?: number | null;
  outputPrice?: number | null;
  isFree: boolean;
  isLocal: boolean;
  supportsVision: boolean;
  supportsTools: boolean;
  supportsReasoning: boolean;
}

export interface McpServerInfo {
  id: string;
  name: string;
  transport: string;
  status: string;
  tools: string[];
}

export interface ContextSnapshot {
  working_dir: string;
  summary: string;
  last_updated: string | null;
  session_count: number;
  memory_size_kb: number;
}

export type PermValue = 'allowed' | 'ask' | 'blocked';

export interface Settings {
  defaultProvider?: string | null;
  defaultModel?: string | null;
  theme?: string | null;
  language?: string | null;
  autoAcceptPermissions?: boolean | null;
  showReasoning?: boolean | null;
  soundEffects?: boolean | null;
  notifications?: boolean | null;
  tokenBudgetPerSession?: number | null;
  autoCompressThreshold?: number | null;
  mcpEnabled?: boolean | null;
  keybindings?: Record<string, string> | null;
  permissions?: Record<string, string> | null;
  expandShellOutput?: boolean | null;
  expandEditOutput?: boolean | null;
  sessionProgressBar?: boolean | null;
  showFileTree?: boolean | null;
  commandPaletteButton?: boolean | null;
  tokenGodAutoCompress?: boolean | null;
  terminalShell?: string | null;
  colorScheme?: string | null;
  uiFont?: string | null;
  codeFont?: string | null;
  customTheme?: CustomTheme | null;
  agentResponseLanguage?: string | null;
  dateFormat?: string | null;
  projectMemoryEnabled?: boolean | null;
  autoSummarizeSessions?: boolean | null;
  userPreferencesEnabled?: boolean | null;
  permissionRead?: PermValue | null;
  permissionWrite?: PermValue | null;
  permissionShell?: PermValue | null;
  permissionNetwork?: PermValue | null;
  permissionDelete?: PermValue | null;
  permissionGit?: PermValue | null;
  permissionMcp?: PermValue | null;
}

export interface CustomTheme {
  bg: string;
  side: string;
  acc: string;
  txt: string;
  brd: string;
}

export interface SessionInfo {
  id: string;
  title: string;
  provider: string;
  model: string;
  created_at: string;
  message_count: number;
}

export interface SessionDetail {
  id: string;
  title: string;
  provider: string;
  model: string;
  created_at: string;
  updated_at: string;
  messages: { role: string; content: string; created_at: string }[];
}

const CORE = '/api';

export async function getProviders(): Promise<ProviderInfo[]> {
  const res = await fetch(`${CORE}/providers`);
  if (!res.ok) return [];
  return res.json();
}

export async function getModels(opts: { free?: boolean } = {}): Promise<ModelInfo[]> {
  const q = new URLSearchParams();
  if (opts.free) q.set('free', 'true');
  const res = await fetch(`${CORE}/models?${q.toString()}`);
  if (!res.ok) return [];
  return res.json();
}

export async function getMcpServers(): Promise<McpServerInfo[]> {
  const res = await fetch(`${CORE}/mcp/servers`);
  if (!res.ok) return [];
  return res.json();
}

export async function addMcpServer(input: {
  id: string;
  name: string;
  transport: string;
}): Promise<McpServerInfo> {
  const res = await fetch(`${CORE}/mcp/servers`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(input),
  });
  if (!res.ok) throw new Error(`addMcpServer failed: ${res.status}`);
  return res.json();
}

export async function getContext(): Promise<ContextSnapshot> {
  const res = await fetch(`${CORE}/context`);
  if (!res.ok)
    return { working_dir: '', summary: '', last_updated: null, session_count: 0, memory_size_kb: 0 };
  return res.json();
}

export async function getSettings(): Promise<Settings> {
  const res = await fetch(`${CORE}/settings`);
  if (!res.ok) return {};
  return res.json();
}

let saveTimer: ReturnType<typeof setTimeout> | null = null;
export async function putSettings(s: Settings, debounceMs = 250): Promise<void> {
  if (saveTimer) clearTimeout(saveTimer);
  return new Promise((resolve, reject) => {
    saveTimer = setTimeout(async () => {
      try {
        const res = await fetch(`${CORE}/settings`, {
          method: 'PUT',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify(s),
        });
        if (!res.ok) throw new Error(`save failed: ${res.status}`);
        resolve();
      } catch (e) {
        reject(e);
      }
    }, debounceMs);
  });
}

export async function listSessions(): Promise<SessionInfo[]> {
  const res = await fetch(`${CORE}/sessions`);
  if (!res.ok) return [];
  return res.json();
}

export async function createSession(input: { title?: string; provider?: string; model?: string }): Promise<SessionInfo> {
  const res = await fetch(`${CORE}/sessions`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(input),
  });
  if (!res.ok) throw new Error(`createSession failed: ${res.status}`);
  return res.json();
}

export async function getSession(id: string): Promise<SessionDetail | null> {
  const res = await fetch(`${CORE}/sessions/${id}`);
  if (!res.ok) return null;
  return res.json();
}

export async function deleteSession(id: string): Promise<void> {
  await fetch(`${CORE}/sessions/${id}`, { method: 'DELETE' });
}

export async function summarizeSession(id: string, provider: string, model: string): Promise<{ summary: string }> {
  const res = await fetch(`${CORE}/sessions/${id}/summarize`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ provider, model }),
  });
  if (!res.ok) throw new Error(`summarize failed: ${res.status}`);
  return res.json();
}

export async function* streamChat(req: {
  provider?: string;
  model?: string;
  message: string;
  sessionId?: string;
  history?: { role: string; content: string }[];
}): AsyncGenerator<string> {
  const res = await fetch(`${CORE}/chat`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok || !res.body) {
    throw new Error(`chat failed: ${res.status}`);
  }
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let idx;
    while ((idx = buffer.indexOf('\n')) >= 0) {
      const line = buffer.slice(0, idx).trimEnd();
      buffer = buffer.slice(idx + 1);
      if (!line.startsWith('data:')) continue;
      const data = line.slice(5).trim();
      if (data === '[DONE]') return;
      if (data.length === 0) continue;
      yield data;
    }
  }
}
