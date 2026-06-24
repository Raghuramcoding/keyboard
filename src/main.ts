import { app, BrowserWindow, ipcMain, dialog, shell } from 'electron';
import { join, dirname, extname, basename, delimiter } from 'path';
import { promises as fsp, existsSync, readFileSync } from 'fs';
import { exec, spawn } from 'child_process';
import { homedir } from 'os';
import { createOpenAICompatible } from '@ai-sdk/openai-compatible';
import { generateText } from 'ai';

const isDev = !app.isPackaged;
const OLLAMA_HOST = process.env.OLLAMA_HOST || 'http://localhost:11434';

// In smoke-test mode, isolate settings to a temp dir so we never clobber real settings.
if (process.env.CLAUDE_CODE_SMOKE === '1') {
  try { app.setPath('userData', join(require('os').tmpdir(), 'claude-code-smoke')); } catch { /* ignore */ }
}

// ─── Optional native PTY (graceful fallback to piped shell) ──────────────────
let pty: any = null;
let ptyAvailable = false;
try {
  pty = require('node-pty');
  ptyAvailable = true;
} catch (e: any) {
  console.warn('[pty] node-pty unavailable, falling back to piped shell:', e?.message);
}

// ─── Settings persistence ────────────────────────────────────────────────────
// A user-configured OpenAI-compatible endpoint (OpenAI, OpenRouter, Groq,
// Together, local LM Studio / llama.cpp, etc.). Talked to via the Vercel AI SDK.
interface CustomProvider {
  id: string;
  name: string;
  baseURL: string;
  apiKey: string;
  models: string[];
}

interface Settings {
  customButtons: { id: string; label: string; command: string; icon?: string }[];
  background: { type: 'none' | 'color' | 'image'; color?: string; imageDataUrl?: string; opacity?: number };
  accentColor: string;
  anthropicApiKey?: string;
  terminalFontSize: number;
  workingDir?: string;
  customProviders: CustomProvider[];
}

const DEFAULT_SETTINGS: Settings = {
  customButtons: [],
  background: { type: 'none', opacity: 0.25 },
  accentColor: '#0e639c',
  terminalFontSize: 13,
  customProviders: [],
};

const settingsPath = () => join(app.getPath('userData'), 'settings.json');

function loadSettings(): Settings {
  try {
    const raw = readFileSync(settingsPath(), 'utf8');
    return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

async function saveSettings(s: Settings): Promise<void> {
  await fsp.mkdir(dirname(settingsPath()), { recursive: true });
  await fsp.writeFile(settingsPath(), JSON.stringify(s, null, 2), 'utf8');
}

// ─── Window ──────────────────────────────────────────────────────────────────
let mainWindow: BrowserWindow | null = null;
const SMOKE = process.env.CLAUDE_CODE_SMOKE === '1';

const createWindow = () => {
  mainWindow = new BrowserWindow({
    width: 1320,
    height: 880,
    minWidth: 940,
    minHeight: 640,
    title: 'Claude Code',
    backgroundColor: '#1e1e1e',
    autoHideMenuBar: true,
    show: !SMOKE,
    webPreferences: {
      preload: join(__dirname, 'preload.js'),
      nodeIntegration: false,
      contextIsolation: true,
    },
  });

  const loaded = mainWindow.loadFile(join(__dirname, 'renderer/index.html'));
  if (isDev && !SMOKE) mainWindow.webContents.openDevTools({ mode: 'detach' });
  if (SMOKE) loaded.then(() => runSmokeTest(mainWindow!));

  // Open external links in the OS browser, never in-app.
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('http')) shell.openExternal(url);
    return { action: 'deny' };
  });

  mainWindow.on('closed', () => { mainWindow = null; });
};

app.whenReady().then(() => {
  createWindow();
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on('window-all-closed', () => {
  killAllPtys();
  if (process.platform !== 'darwin') app.quit();
});

// ═══════════════════════════════════════════════════════════════════════════
//  AI providers (native Node — no Python backend required)
// ═══════════════════════════════════════════════════════════════════════════

async function ollamaGenerate(model: string, prompt: string, options: any): Promise<any> {
  try {
    const res = await fetch(`${OLLAMA_HOST}/api/generate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model,
        prompt,
        stream: false,
        options: {
          temperature: options.temperature ?? 0.7,
          top_p: options.top_p ?? 0.9,
          top_k: options.top_k ?? 40,
        },
        ...(options.system ? { system: options.system } : {}),
      }),
    });
    if (!res.ok) return { error: `Ollama API error: ${res.status}` };
    return await res.json();
  } catch (e: any) {
    return { error: `Cannot connect to Ollama (${OLLAMA_HOST}). Is it running? ${e.message}` };
  }
}

function getApiKey(): string {
  return loadSettings().anthropicApiKey || process.env.ANTHROPIC_API_KEY || '';
}

async function claudeGenerate(model: string, prompt: string, options: any): Promise<any> {
  const apiKey = getApiKey();
  if (!apiKey) return { error: 'Claude API not configured. Add an Anthropic API key in Settings (or set ANTHROPIC_API_KEY).' };
  try {
    const body: any = {
      model: model || 'claude-opus-4-8',
      max_tokens: options.max_tokens ?? 4096,
      messages: [{ role: 'user', content: prompt }],
    };
    if (options.system) body.system = options.system;
    if (typeof options.temperature === 'number') body.temperature = options.temperature;

    const res = await fetch('https://api.anthropic.com/v1/messages', {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'x-api-key': apiKey,
        'anthropic-version': '2023-06-01',
      },
      body: JSON.stringify(body),
    });
    const data: any = await res.json();
    if (!res.ok) return { error: `Claude API error: ${data?.error?.message || res.status}` };
    const text = (data.content || []).filter((b: any) => b.type === 'text').map((b: any) => b.text).join('');
    return { response: text, model: data.model, provider: 'claude', usage: data.usage };
  } catch (e: any) {
    return { error: `Claude API request failed: ${e.message}` };
  }
}

// Generate via any OpenAI-compatible endpoint using the Vercel AI SDK.
async function openAICompatibleGenerate(cp: CustomProvider, model: string, prompt: string, options: any): Promise<any> {
  if (!cp.baseURL) return { error: `Provider "${cp.name}" has no base URL configured.` };
  try {
    const provider = createOpenAICompatible({ name: cp.name || 'custom', baseURL: cp.baseURL, apiKey: cp.apiKey || '' });
    const res = await generateText({
      model: provider(model),
      prompt,
      ...(typeof options.temperature === 'number' ? { temperature: options.temperature } : {}),
      ...(options.max_tokens ? { maxOutputTokens: options.max_tokens } : {}),
      ...(options.system ? { system: options.system } : {}),
    });
    return { response: res.text, model, provider: cp.id, usage: res.usage };
  } catch (e: any) {
    return { error: `${cp.name} request failed: ${e?.message || e}` };
  }
}

ipcMain.handle('ai:generate', async (_e, { provider, model, prompt, ...options }: any) => {
  const p = (provider || 'ollama').toLowerCase();
  if (p === 'claude') return claudeGenerate(model, prompt, options);
  if (p === 'ollama') return ollamaGenerate(model, prompt, options);
  // Otherwise treat the provider id as a configured OpenAI-compatible provider.
  const cp = loadSettings().customProviders.find((c) => c.id === provider);
  if (cp) return openAICompatibleGenerate(cp, model, prompt, options);
  return { error: `Unknown provider: ${provider}` };
});

const CLAUDE_MODELS = [
  { name: 'claude-opus-4-8', provider: 'claude', modified_at: '', size: 0, digest: 'claude-opus-4-8', details: { parameter_size: 'Opus 4.8' } },
  { name: 'claude-sonnet-4-6', provider: 'claude', modified_at: '', size: 0, digest: 'claude-sonnet-4-6', details: { parameter_size: 'Sonnet 4.6' } },
  { name: 'claude-haiku-4-5', provider: 'claude', modified_at: '', size: 0, digest: 'claude-haiku-4-5', details: { parameter_size: 'Haiku 4.5' } },
];

ipcMain.handle('models:get', async () => {
  const models: any[] = [];
  try {
    const res = await fetch(`${OLLAMA_HOST}/api/tags`, { signal: AbortSignal.timeout(4000) });
    if (res.ok) {
      const data: any = await res.json();
      for (const m of data.models || []) { m.provider = 'ollama'; models.push(m); }
    }
  } catch { /* Ollama not running — that's fine */ }
  if (getApiKey()) models.push(...CLAUDE_MODELS);
  // OpenAI-compatible custom providers
  for (const cp of loadSettings().customProviders || []) {
    for (const m of cp.models || []) {
      models.push({ name: m, provider: cp.id, modified_at: '', size: 0, digest: m, details: { family: cp.name } });
    }
  }
  return { models };
});

// Discover models from an OpenAI-compatible endpoint (GET /v1/models).
ipcMain.handle('providers:listModels', async (_e, { baseURL, apiKey }: { baseURL: string; apiKey?: string }) => {
  if (!baseURL) return { error: 'No base URL' };
  try {
    const url = baseURL.replace(/\/+$/, '') + '/models';
    const res = await fetch(url, { headers: apiKey ? { Authorization: `Bearer ${apiKey}` } : {}, signal: AbortSignal.timeout(10000) });
    if (!res.ok) return { error: `HTTP ${res.status} from ${url}` };
    const data: any = await res.json();
    const list = data.data || data.models || [];
    const ids = list.map((m: any) => (typeof m === 'string' ? m : m.id || m.name)).filter(Boolean);
    return { models: ids };
  } catch (e: any) {
    return { error: e.message };
  }
});

// models.dev catalog — providers + models with metadata (cached in-memory).
let catalogCache: any = null;
ipcMain.handle('catalog:models', async () => {
  if (catalogCache) return { data: catalogCache };
  try {
    const res = await fetch('https://models.dev/api.json', { signal: AbortSignal.timeout(15000) });
    if (!res.ok) return { error: `models.dev returned HTTP ${res.status}` };
    catalogCache = await res.json();
    return { data: catalogCache };
  } catch (e: any) {
    return { error: `Could not reach models.dev: ${e.message}` };
  }
});

// ─── Ollama registry / model management ──────────────────────────────────────
const KNOWN_OLLAMA_MODELS = [
  'llama3.1', 'llama3.2', 'llama3', 'qwen2.5-coder', 'qwen2.5', 'qwen3', 'codellama',
  'codegemma', 'deepseek-coder-v2', 'deepseek-r1', 'mistral', 'mixtral', 'phi4', 'phi3',
  'gemma2', 'gemma3', 'starcoder2', 'granite-code', 'yi-coder', 'wizardcoder', 'stable-code',
  'tinyllama', 'nomic-embed-text', 'llava', 'command-r',
];

async function localOllamaNames(): Promise<Set<string>> {
  try {
    const res = await fetch(`${OLLAMA_HOST}/api/tags`, { signal: AbortSignal.timeout(4000) });
    if (res.ok) {
      const data: any = await res.json();
      return new Set((data.models || []).map((m: any) => String(m.name).split(':')[0]));
    }
  } catch { /* ignore */ }
  return new Set();
}

ipcMain.handle('backend:action', async (_e, payload: any) => {
  const action = payload?.action;
  try {
    switch (action) {
      case 'write_file': {
        const p = payload.path;
        if (!p) return { error: 'Missing path' };
        await fsp.mkdir(dirname(p), { recursive: true });
        await fsp.writeFile(p, payload.content ?? '', 'utf8');
        return { success: true, path: p };
      }
      case 'list_registry': {
        const search = String(payload.search || '').toLowerCase();
        const local = await localOllamaNames();
        const models = KNOWN_OLLAMA_MODELS.filter(m => !search || m.includes(search)).map(name => ({
          name, installed: [...local].some(l => l === name || l.includes(name)), tags: ['latest'],
        }));
        return { models };
      }
      case 'pull_model': {
        const res = await fetch(`${OLLAMA_HOST}/api/pull`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ name: payload.model, stream: false }),
        });
        if (!res.ok) return { error: `Failed to pull model: ${res.status}` };
        return { success: true, model: payload.model };
      }
      case 'delete_model': {
        const res = await fetch(`${OLLAMA_HOST}/api/delete`, {
          method: 'DELETE', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ name: payload.model }),
        });
        if (!res.ok) return { error: `Failed to delete model: ${res.status}` };
        return { success: true, model: payload.model };
      }
      case 'model_info': {
        const res = await fetch(`${OLLAMA_HOST}/api/show`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ name: payload.model }),
        });
        if (!res.ok) return { error: `Model not found: ${payload.model}` };
        return { model: payload.model, info: await res.json() };
      }
      default:
        return { error: `Unknown action: ${action}` };
    }
  } catch (e: any) {
    return { error: e.message };
  }
});

// ─── One-shot command execution (git, scaffold helpers, etc.) ────────────────
ipcMain.handle('execute:command', async (_e, { command, cwd }: { command: string; cwd?: string }) => {
  return new Promise((resolve) => {
    exec(command, {
      cwd: cwd || loadSettings().workingDir || process.cwd(),
      timeout: 60000,
      windowsHide: true,
      shell: process.platform === 'win32' ? 'powershell.exe' : '/bin/bash',
      maxBuffer: 10 * 1024 * 1024,
    }, (err, stdout, stderr) => {
      resolve({ stdout: stdout || '', stderr: stderr || (err && !stdout && !stderr ? err.message : ''), returncode: err ? (err as any).code || 1 : 0 });
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
//  Terminal — real PTY via node-pty (fallback: piped shell)
// ═══════════════════════════════════════════════════════════════════════════
const ptys = new Map<string, any>();

function defaultShell(): { file: string; args: string[] } {
  if (process.platform === 'win32') return { file: 'powershell.exe', args: [] };
  return { file: process.env.SHELL || '/bin/bash', args: [] };
}

function killAllPtys() {
  for (const [, p] of ptys) { try { p.kill(); } catch { /* ignore */ } }
  ptys.clear();
}

ipcMain.handle('pty:create', (_e, { id, cwd, cols, rows }: { id: string; cwd?: string; cols?: number; rows?: number }) => {
  const wd = cwd || loadSettings().workingDir || homedir();
  const { file, args } = defaultShell();
  if (ptyAvailable) {
    const proc = pty.spawn(file, args, {
      name: 'xterm-256color', cols: cols || 80, rows: rows || 24, cwd: wd, env: process.env,
    });
    proc.onData((data: string) => mainWindow?.webContents.send('pty:data', { id, data }));
    proc.onExit(({ exitCode }: any) => { mainWindow?.webContents.send('pty:exit', { id, code: exitCode }); ptys.delete(id); });
    ptys.set(id, proc);
    return { ok: true, mode: 'pty', shell: file };
  }
  // Fallback: piped shell (line-oriented, no full TUI)
  const proc = spawn(file, args, { cwd: wd, env: process.env, windowsHide: true });
  proc.stdout.on('data', (d: Buffer) => mainWindow?.webContents.send('pty:data', { id, data: d.toString() }));
  proc.stderr.on('data', (d: Buffer) => mainWindow?.webContents.send('pty:data', { id, data: d.toString() }));
  proc.on('exit', (code) => { mainWindow?.webContents.send('pty:exit', { id, code }); ptys.delete(id); });
  (proc as any)._piped = true;
  ptys.set(id, proc);
  return { ok: true, mode: 'piped', shell: file };
});

const smokePtyWrites: { id: string; data: string }[] = [];

ipcMain.on('pty:write', (_e, { id, data }: { id: string; data: string }) => {
  if (process.env.CLAUDE_CODE_SMOKE === '1') smokePtyWrites.push({ id, data });
  const p = ptys.get(id);
  if (!p) return;
  if ((p as any)._piped) p.stdin.write(data);
  else p.write(data);
});

ipcMain.on('pty:resize', (_e, { id, cols, rows }: { id: string; cols: number; rows: number }) => {
  const p = ptys.get(id);
  if (p && !(p as any)._piped) { try { p.resize(cols, rows); } catch { /* ignore */ } }
});

ipcMain.on('pty:kill', (_e, { id }: { id: string }) => {
  const p = ptys.get(id);
  if (p) { try { p.kill(); } catch { /* ignore */ } ptys.delete(id); }
});

// ═══════════════════════════════════════════════════════════════════════════
//  AI coding-agent auto-detection
// ═══════════════════════════════════════════════════════════════════════════
interface AgentDef { id: string; label: string; command: string; icon: string; runArgs?: string }

const KNOWN_AGENTS: AgentDef[] = [
  { id: 'claude', label: 'Claude Code', command: 'claude', icon: '⚡' },
  { id: 'codex', label: 'Codex CLI', command: 'codex', icon: '🧠' },
  { id: 'gemini', label: 'Gemini CLI', command: 'gemini', icon: '✨' },
  { id: 'aider', label: 'Aider', command: 'aider', icon: '🤝' },
  { id: 'opencode', label: 'OpenCode', command: 'opencode', icon: '📟' },
  { id: 'cursor-agent', label: 'Cursor Agent', command: 'cursor-agent', icon: '⌨️' },
  { id: 'copilot', label: 'Copilot CLI', command: 'copilot', icon: '🐙' },
  { id: 'ollama', label: 'Ollama', command: 'ollama', icon: '🦙', runArgs: 'list' },
];

// Resolve a command on PATH (Windows: try common executable extensions).
function resolveOnPath(cmd: string): string | null {
  const paths = (process.env.PATH || '').split(delimiter).filter(Boolean);
  const exts = process.platform === 'win32'
    ? (process.env.PATHEXT || '.COM;.EXE;.BAT;.CMD;.PS1').split(';')
    : [''];
  for (const dir of paths) {
    for (const ext of exts) {
      const full = join(dir, cmd + ext);
      try { if (existsSync(full)) return full; } catch { /* ignore */ }
    }
  }
  return null;
}

ipcMain.handle('agents:detect', async () => {
  const found = KNOWN_AGENTS.map(a => {
    const path = resolveOnPath(a.command);
    return { ...a, installed: !!path, path: path || undefined };
  });
  return { agents: found };
});

// ═══════════════════════════════════════════════════════════════════════════
//  Settings + dialogs
// ═══════════════════════════════════════════════════════════════════════════
ipcMain.handle('settings:get', () => loadSettings());
ipcMain.handle('settings:set', async (_e, s: Settings) => { await saveSettings(s); return { ok: true }; });

ipcMain.handle('dialog:pickImage', async () => {
  if (!mainWindow) return { canceled: true };
  const r = await dialog.showOpenDialog(mainWindow, {
    properties: ['openFile'],
    filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'] }],
  });
  if (r.canceled || !r.filePaths[0]) return { canceled: true };
  const buf = await fsp.readFile(r.filePaths[0]);
  const ext = extname(r.filePaths[0]).slice(1).toLowerCase();
  const mime = ext === 'jpg' ? 'jpeg' : ext;
  return { canceled: false, dataUrl: `data:image/${mime};base64,${buf.toString('base64')}`, path: r.filePaths[0] };
});

ipcMain.handle('dialog:pickDirectory', async () => {
  if (!mainWindow) return { canceled: true };
  const r = await dialog.showOpenDialog(mainWindow, { properties: ['openDirectory', 'createDirectory'] });
  if (r.canceled || !r.filePaths[0]) return { canceled: true };
  return { canceled: false, path: r.filePaths[0] };
});

ipcMain.handle('dialog:openFile', async () => {
  if (!mainWindow) return { canceled: true };
  const r = await dialog.showOpenDialog(mainWindow, { properties: ['openFile'] });
  if (r.canceled || !r.filePaths[0]) return { canceled: true };
  const content = await fsp.readFile(r.filePaths[0], 'utf8');
  return { canceled: false, path: r.filePaths[0], name: basename(r.filePaths[0]), content };
});

ipcMain.handle('dialog:saveFile', async (_e, { content, defaultName }: { content: string; defaultName?: string }) => {
  if (!mainWindow) return { canceled: true };
  const r = await dialog.showSaveDialog(mainWindow, { defaultPath: defaultName || 'untitled.txt' });
  if (r.canceled || !r.filePath) return { canceled: true };
  await fsp.writeFile(r.filePath, content, 'utf8');
  return { canceled: false, path: r.filePath };
});

ipcMain.handle('app:info', () => ({
  version: app.getVersion(),
  ptyAvailable,
  platform: process.platform,
  userData: app.getPath('userData'),
}));

// ─── Headless smoke test (CLAUDE_CODE_SMOKE=1) ───────────────────────────────
async function runSmokeTest(win: BrowserWindow) {
  const errors: string[] = [];
  win.webContents.on('console-message', (_e, level, msg) => { if (level >= 3) errors.push(msg); });
  try {
    await new Promise((r) => setTimeout(r, 4500)); // let React + Monaco + terminal mount
    const report = await win.webContents.executeJavaScript(`(() => {
      const root = document.getElementById('root');
      const text = document.body.innerText || '';
      return {
        rootChildren: root ? root.children.length : -1,
        hasTitle: text.includes('Claude Code'),
        monaco: !!document.querySelector('.monaco-editor'),
        xterm: !!document.querySelector('.xterm'),
        terminalDock: text.includes('TERMINAL'),
      };
    })()`);
    // Also probe a real PTY end-to-end (the packaging-specific risk).
    let ptyRan = false;
    try {
      const id = 'smoke-pty';
      await win.webContents.executeJavaScript('true');
      const proc = ptyAvailable
        ? pty.spawn(process.platform === 'win32' ? 'powershell.exe' : 'bash', [], { name: 'xterm-color', cols: 80, rows: 24, cwd: process.cwd(), env: process.env })
        : null;
      if (proc) {
        let buf = '';
        proc.onData((d: string) => { buf += d; });
        proc.write('echo SMOKE_PTY_MARKER\r\n');
        await new Promise((r) => setTimeout(r, 1500));
        ptyRan = buf.includes('SMOKE_PTY_MARKER');
        proc.kill();
      }
    } catch { /* ptyRan stays false */ }

    // Functional test: launching two agents in a row must run BOTH (the second
    // in its own tab, not typed into the first's running session).
    let twoAgents = false; let tabCount = 0;
    try {
      await win.webContents.executeJavaScript(`window.__terminal && window.__terminal.current && window.__terminal.current.launchCommand('echo SMOKE_AGENT_AAA')`);
      await new Promise((r) => setTimeout(r, 900));
      await win.webContents.executeJavaScript(`window.__terminal.current.launchCommand('echo SMOKE_AGENT_BBB')`);
      await new Promise((r) => setTimeout(r, 2600));
      tabCount = await win.webContents.executeJavaScript(`document.querySelectorAll('.xterm').length`);
      const launches = smokePtyWrites.filter((w) => /SMOKE_AGENT_(AAA|BBB)/.test(w.data));
      const ids = new Set(launches.map((l) => l.id));
      twoAgents = ids.size >= 2 && launches.some((l) => l.data.includes('SMOKE_AGENT_AAA')) && launches.some((l) => l.data.includes('SMOKE_AGENT_BBB'));
    } catch { /* twoAgents stays false */ }

    // OpenAI-compatible provider test: stand up a fake endpoint, configure a
    // provider against it, and verify both model listing and real generation
    // through the Vercel AI SDK.
    let providerListed = false; let providerGen = false;
    try {
      const http = require('http');
      const srv = http.createServer((req: any, res: any) => {
        let body = ''; req.on('data', (c: any) => (body += c));
        req.on('end', () => {
          res.setHeader('content-type', 'application/json');
          if (req.url.includes('/models')) { res.end(JSON.stringify({ data: [{ id: 'smoke-model' }] })); return; }
          res.end(JSON.stringify({ id: 'x', object: 'chat.completion', model: 'smoke-model', choices: [{ index: 0, message: { role: 'assistant', content: 'SMOKE_PROVIDER_REPLY' }, finish_reason: 'stop' }], usage: { prompt_tokens: 1, completion_tokens: 1, total_tokens: 2 } }));
        });
      });
      await new Promise<void>((r) => srv.listen(0, r));
      const port = (srv.address() as any).port;
      const sset = loadSettings();
      sset.customProviders = [{ id: 'cp-smoke', name: 'SmokeAI', baseURL: `http://localhost:${port}/v1`, apiKey: 'x', models: ['smoke-model'] }];
      await saveSettings(sset);

      providerListed = await win.webContents.executeJavaScript(`window.electronAPI.getModels().then(r => (r.models||[]).some(m => m.provider==='cp-smoke' && m.name==='smoke-model'))`);
      const gen = await win.webContents.executeJavaScript(`window.electronAPI.aiGenerate('cp-smoke','smoke-model','hi').then(r => r.response || ('ERR:'+r.error))`);
      providerGen = typeof gen === 'string' && gen.includes('SMOKE_PROVIDER_REPLY');
      srv.close();
    } catch { /* provider flags stay false */ }

    const ok = report.rootChildren > 0 && report.hasTitle && report.monaco && report.xterm
      && (!ptyAvailable || ptyRan) && twoAgents && tabCount >= 2 && providerListed && providerGen;
    const out = { ...report, ptyAvailable, ptyRan, twoAgents, tabCount, providerListed, providerGen, errors: errors.slice(0, 10), result: ok ? 'PASS' : 'FAIL' };
    console.log('SMOKE_REPORT ' + JSON.stringify(out));
    console.log('SMOKE_RESULT ' + out.result);
    const outFile = process.env.CLAUDE_CODE_SMOKE_OUT;
    if (outFile) { try { require('fs').writeFileSync(outFile, JSON.stringify(out)); } catch { /* ignore */ } }
    app.exit(ok ? 0 : 1);
  } catch (e: any) {
    const outFile = process.env.CLAUDE_CODE_SMOKE_OUT;
    if (outFile) { try { require('fs').writeFileSync(outFile, JSON.stringify({ result: 'FAIL', error: e.message })); } catch { /* ignore */ } }
    console.log('SMOKE_RESULT FAIL ' + e.message);
    app.exit(1);
  }
}
