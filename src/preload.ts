// Preload: the only bridge between the sandboxed renderer and Node/Electron.
const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
  // ── AI ─────────────────────────────────────────────────────────────────────
  aiGenerate: (provider: string, model: string, prompt: string, options?: any) =>
    ipcRenderer.invoke('ai:generate', { provider, model, prompt, ...(options || {}) }),
  getModels: () => ipcRenderer.invoke('models:get'),

  // ── Model management ─────────────────────────────────────────────────────────
  backendAction: (action: string, data?: any) => ipcRenderer.invoke('backend:action', { action, ...(data || {}) }),
  pullModel: (model: string) => ipcRenderer.invoke('backend:action', { action: 'pull_model', model }),
  deleteModel: (model: string) => ipcRenderer.invoke('backend:action', { action: 'delete_model', model }),
  listRegistry: (search?: string) => ipcRenderer.invoke('backend:action', { action: 'list_registry', search: search || '' }),
  getModelInfo: (model: string) => ipcRenderer.invoke('backend:action', { action: 'model_info', model }),

  // ── Command execution (one-shot, for git/scaffold) ──────────────────────────
  executeCommand: (command: string, cwd?: string) => ipcRenderer.invoke('execute:command', { command, cwd }),

  // ── Terminal (PTY) ──────────────────────────────────────────────────────────
  ptyCreate: (opts: { id: string; cwd?: string; cols?: number; rows?: number }) => ipcRenderer.invoke('pty:create', opts),
  ptyWrite: (id: string, data: string) => ipcRenderer.send('pty:write', { id, data }),
  ptyResize: (id: string, cols: number, rows: number) => ipcRenderer.send('pty:resize', { id, cols, rows }),
  ptyKill: (id: string) => ipcRenderer.send('pty:kill', { id }),
  onPtyData: (cb: (payload: { id: string; data: string }) => void) => {
    const fn = (_e: any, payload: any) => cb(payload);
    ipcRenderer.on('pty:data', fn);
    return () => ipcRenderer.removeListener('pty:data', fn);
  },
  onPtyExit: (cb: (payload: { id: string; code: number }) => void) => {
    const fn = (_e: any, payload: any) => cb(payload);
    ipcRenderer.on('pty:exit', fn);
    return () => ipcRenderer.removeListener('pty:exit', fn);
  },

  // ── Agents ──────────────────────────────────────────────────────────────────
  detectAgents: () => ipcRenderer.invoke('agents:detect'),

  // ── OpenAI-compatible providers ─────────────────────────────────────────────
  listProviderModels: (baseURL: string, apiKey?: string) => ipcRenderer.invoke('providers:listModels', { baseURL, apiKey }),
  getModelCatalog: () => ipcRenderer.invoke('catalog:models'),

  // ── Settings ────────────────────────────────────────────────────────────────
  getSettings: () => ipcRenderer.invoke('settings:get'),
  setSettings: (s: any) => ipcRenderer.invoke('settings:set', s),

  // ── Dialogs ─────────────────────────────────────────────────────────────────
  pickImage: () => ipcRenderer.invoke('dialog:pickImage'),
  pickDirectory: () => ipcRenderer.invoke('dialog:pickDirectory'),
  openFileDialog: () => ipcRenderer.invoke('dialog:openFile'),
  saveFileDialog: (content: string, defaultName?: string) => ipcRenderer.invoke('dialog:saveFile', { content, defaultName }),

  appInfo: () => ipcRenderer.invoke('app:info'),
});

contextBridge.exposeInMainWorld('versions', {
  node: () => process.versions.node,
  chrome: () => process.versions.chrome,
  electron: () => process.versions.electron,
});
