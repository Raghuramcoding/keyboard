import React, { useState, useEffect, useRef, useCallback } from 'react';
import * as monaco from 'monaco-editor';
import TerminalPane, { TerminalHandle } from './Terminal';
import SettingsModal, { AppSettings } from './Settings';

// ─── Types ───────────────────────────────────────────────────────────────────
interface ModelInfo {
  name: string; provider: string; modified_at: string; size: number; digest: string;
  details?: { family?: string; parameter_size?: string; quantization_level?: string };
}
interface ChatMessage { role: 'user' | 'assistant'; content: string; timestamp: Date; provider?: string; model?: string; }
interface RegistryModel { name: string; installed: boolean; tags: string[]; }
interface GitStatus { branch: string; changes: number; staged: number; files: { path: string; status: string; staged: boolean }[]; ahead?: number; behind?: number; }
interface ScaffoldTemplate { name: string; description: string; language: string; files: { path: string; content: string }[]; }
interface ModelsResponse { models: ModelInfo[]; }
interface AgentInfo { id: string; label: string; command: string; icon: string; runArgs?: string; installed: boolean; path?: string; }

declare global {
  interface Window {
    electronAPI: {
      aiGenerate: (provider: string, model: string, prompt: string, options?: any) => Promise<any>;
      getModels: () => Promise<ModelsResponse>;
      executeCommand: (command: string, cwd?: string) => Promise<{ stdout: string; stderr: string; returncode: number }>;
      backendAction: (action: string, data?: any) => Promise<any>;
      pullModel: (name: string) => Promise<any>;
      deleteModel: (name: string) => Promise<any>;
      listRegistry: (search?: string) => Promise<{ models: RegistryModel[] }>;
      getModelInfo: (name: string) => Promise<any>;
      ptyCreate: (opts: { id: string; cwd?: string; cols?: number; rows?: number }) => Promise<any>;
      ptyWrite: (id: string, data: string) => void;
      ptyResize: (id: string, cols: number, rows: number) => void;
      ptyKill: (id: string) => void;
      onPtyData: (cb: (p: { id: string; data: string }) => void) => () => void;
      onPtyExit: (cb: (p: { id: string; code: number }) => void) => () => void;
      detectAgents: () => Promise<{ agents: AgentInfo[] }>;
      listProviderModels: (baseURL: string, apiKey?: string) => Promise<{ models?: string[]; error?: string }>;
      getModelCatalog: () => Promise<{ data?: any; error?: string }>;
      getSettings: () => Promise<AppSettings>;
      setSettings: (s: AppSettings) => Promise<any>;
      pickImage: () => Promise<{ canceled: boolean; dataUrl?: string; path?: string }>;
      pickDirectory: () => Promise<{ canceled: boolean; path?: string }>;
      openFileDialog: () => Promise<{ canceled: boolean; path?: string; name?: string; content?: string }>;
      saveFileDialog: (content: string, defaultName?: string) => Promise<{ canceled: boolean; path?: string }>;
      appInfo: () => Promise<{ version: string; ptyAvailable: boolean; platform: string; userData: string }>;
    };
  }
}

// ─── Constants ───────────────────────────────────────────────────────────────
const PROVIDER_COLORS: Record<string, string> = { ollama: '#e67e22', claude: '#d4a574' };
const PROVIDER_NAMES: Record<string, string> = { ollama: 'Ollama', claude: 'Claude' };

const SCAFFOLD_TEMPLATES: ScaffoldTemplate[] = [
  {
    name: 'React + TypeScript App', description: 'A modern React app with TypeScript and Vite', language: 'typescript',
    files: [
      { path: 'package.json', content: JSON.stringify({ name: 'my-app', version: '1.0.0', private: true, scripts: { dev: 'vite', build: 'tsc && vite build', preview: 'vite preview' } }, null, 2) },
      { path: 'src/App.tsx', content: 'import React from "react";\n\nconst App: React.FC = () => <div>Hello React!</div>;\n\nexport default App;\n' },
      { path: 'src/main.tsx', content: 'import React from "react";\nimport ReactDOM from "react-dom/client";\nimport App from "./App";\n\nReactDOM.createRoot(document.getElementById("root")!).render(<App />);\n' },
      { path: 'index.html', content: '<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8" /><title>My App</title></head><body><div id="root"></div><script type="module" src="/src/main.tsx"></script></body></html>' },
      { path: 'tsconfig.json', content: JSON.stringify({ compilerOptions: { target: 'ES2020', module: 'ESNext', jsx: 'react-jsx', strict: true }, include: ['src'] }, null, 2) },
    ],
  },
  {
    name: 'Python CLI App', description: 'Python CLI with argparse, tests, and packaging', language: 'python',
    files: [
      { path: 'pyproject.toml', content: '[build-system]\nrequires = ["setuptools"]\nbuild-backend = "setuptools.build_meta"\n\n[project]\nname = "my-cli"\nversion = "0.1.0"\n' },
      { path: 'src/cli.py', content: 'import argparse\n\ndef main():\n    parser = argparse.ArgumentParser()\n    parser.add_argument("--name", default="World")\n    args = parser.parse_args()\n    print(f"Hello, {args.name}!")\n\nif __name__ == "__main__":\n    main()\n' },
      { path: 'tests/test_cli.py', content: 'from src.cli import main\n\ndef test_main():\n    assert True\n' },
      { path: 'README.md', content: '# My CLI App\n\nA Python CLI application.\n' },
    ],
  },
  {
    name: 'Node.js Express API', description: 'REST API with Express and TypeScript', language: 'typescript',
    files: [
      { path: 'package.json', content: JSON.stringify({ name: 'my-api', version: '1.0.0', scripts: { dev: 'tsx watch src/index.ts', build: 'tsc', start: 'node dist/index.js' }, dependencies: { express: '^4.18.0' }, devDependencies: { '@types/express': '^4.17.0', typescript: '^5.0.0', tsx: '^4.0.0' } }, null, 2) },
      { path: 'src/index.ts', content: 'import express from "express";\n\nconst app = express();\nconst port = process.env.PORT || 3000;\n\napp.get("/", (_, res) => res.json({ message: "Hello API" }));\n\napp.listen(port, () => console.log(`Server running on :${port}`));\n' },
      { path: 'tsconfig.json', content: JSON.stringify({ compilerOptions: { target: 'ES2020', module: 'commonjs', outDir: './dist', strict: true } }, null, 2) },
    ],
  },
  {
    name: 'Rust CLI Tool', description: 'A Rust CLI with clap for argument parsing', language: 'rust',
    files: [
      { path: 'Cargo.toml', content: '[package]\nname = "my-tool"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\nclap = { version = "4", features = ["derive"] }\n' },
      { path: 'src/main.rs', content: 'use clap::Parser;\n\n#[derive(Parser)]\nstruct Args {\n    name: String,\n}\n\nfn main() {\n    let args = Args::parse();\n    println!("Hello, {}!", args.name);\n}\n' },
    ],
  },
];

// ─── Helpers ─────────────────────────────────────────────────────────────────
const formatSize = (bytes: number): string => {
  if (!bytes) return '';
  const u = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${u[i]}`;
};
const formatTime = (d: Date) => d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
const detectLang = (name: string): string => {
  const map: Record<string, string> = {
    ts: 'typescript', tsx: 'typescript', js: 'javascript', jsx: 'javascript', py: 'python', rs: 'rust',
    go: 'go', java: 'java', cs: 'csharp', cpp: 'cpp', c: 'c', html: 'html', css: 'css', json: 'json',
    md: 'markdown', yaml: 'yaml', yml: 'yaml', sql: 'sql', sh: 'shell',
  };
  return map[name.split('.').pop()?.toLowerCase() || ''] || 'plaintext';
};

const DEFAULT_SETTINGS: AppSettings = {
  customButtons: [], background: { type: 'none', opacity: 0.25 }, accentColor: '#0e639c', terminalFontSize: 13, customProviders: [],
};

type SidebarTab = 'models' | 'git' | 'scaffold';

// ─── App ─────────────────────────────────────────────────────────────────────
const App: React.FC = () => {
  const editorRef = useRef<HTMLDivElement>(null);
  const chatEndRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<TerminalHandle>(null);

  const [sidebarTab, setSidebarTab] = useState<SidebarTab>('models');

  // Settings / agents / workspace
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [showSettings, setShowSettings] = useState(false);
  const [agents, setAgents] = useState<AgentInfo[]>([]);
  const [showTerminal, setShowTerminal] = useState(true);
  const [terminalHeight, setTerminalHeight] = useState(260);
  const [ptyMode, setPtyMode] = useState<'pty' | 'piped' | 'unknown'>('unknown');

  // Models
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [selectedProvider, setSelectedProvider] = useState('ollama');
  const [selectedModel, setSelectedModel] = useState('');
  const [isLoadingModels, setIsLoadingModels] = useState(true);
  const [modelsError, setModelsError] = useState<string | null>(null);
  const [temperature, setTemperature] = useState(0.7);

  // Model management
  const [showModelManager, setShowModelManager] = useState(false);
  const [registryModels, setRegistryModels] = useState<RegistryModel[]>([]);
  const [registrySearch, setRegistrySearch] = useState('');
  const [isPulling, setIsPulling] = useState<string | null>(null);
  const [pullStatus, setPullStatus] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState<string | null>(null);

  const filteredModels = selectedProvider === 'all' ? models : models.filter(m => m.provider === selectedProvider);

  // Chat
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [inputValue, setInputValue] = useState('');
  const [isGenerating, setIsGenerating] = useState(false);

  // Editor
  const [editor, setEditor] = useState<monaco.editor.IStandaloneCodeEditor | null>(null);
  const [activeFile, setActiveFile] = useState('untitled.ts');

  // Git
  const [gitStatus, setGitStatus] = useState<GitStatus | null>(null);
  const [gitLoading, setGitLoading] = useState(false);
  const [commitMsg, setCommitMsg] = useState('');
  const [gitBranchInput, setGitBranchInput] = useState('');
  const [gitDiff, setGitDiff] = useState('');

  // Scaffold
  const [scaffoldTarget, setScaffoldTarget] = useState('');
  const [scaffolding, setScaffolding] = useState(false);

  const workingDir = settings.workingDir;

  useEffect(() => { chatEndRef.current?.scrollIntoView({ behavior: 'smooth' }); }, [chatMessages]);
  // Expose the terminal handle for the headless smoke test (harmless debug handle).
  useEffect(() => { (window as any).__terminal = terminalRef; }, []);

  // ── Load settings + agents + app info ──────────────────────────────────────
  useEffect(() => {
    (async () => {
      try { const s = await window.electronAPI.getSettings(); if (s) setSettings({ ...DEFAULT_SETTINGS, ...s }); } catch { /* ignore */ }
      try { const a = await window.electronAPI.detectAgents(); setAgents(a.agents || []); } catch { /* ignore */ }
      try { const info = await window.electronAPI.appInfo(); setPtyMode(info.ptyAvailable ? 'pty' : 'piped'); } catch { /* ignore */ }
    })();
  }, []);

  // ── Apply theme (accent + background) via CSS variables ────────────────────
  useEffect(() => {
    const root = document.documentElement.style;
    root.setProperty('--accent', settings.accentColor);
    const bg = settings.background;
    if (bg.type === 'none') {
      root.setProperty('--bg-base', '#1e1e1e');
      root.setProperty('--bg-panel', '#252526');
      root.setProperty('--bg-elevated', '#2d2d2d');
    } else {
      const op = bg.opacity ?? 0.25;
      const a = Math.max(0.12, Math.min(1, 1 - op));
      root.setProperty('--bg-base', `rgba(20,20,22,${Math.max(0, a - 0.12)})`);
      root.setProperty('--bg-panel', `rgba(37,37,38,${a})`);
      root.setProperty('--bg-elevated', `rgba(45,45,48,${a})`);
    }
  }, [settings]);

  const saveSettings = async (s: AppSettings) => {
    setSettings(s);
    setShowSettings(false);
    try { await window.electronAPI.setSettings(s); } catch { /* ignore */ }
    // Re-detect agents in case PATH-affecting changes; refresh models if API key changed.
    fetchModels();
  };

  // ── Models ─────────────────────────────────────────────────────────────────
  const fetchModels = useCallback(async () => {
    setIsLoadingModels(true);
    setModelsError(null);
    try {
      const r = await window.electronAPI.getModels();
      if (r.models?.length) {
        setModels(r.models);
        const om = r.models.filter(m => m.provider === 'ollama');
        if (om.length) { setSelectedModel(om[0].name); setSelectedProvider('ollama'); }
        else { setSelectedModel(r.models[0].name); setSelectedProvider(r.models[0].provider); }
      } else { setModels([]); setModelsError('No models found. Start Ollama or add a Claude API key in Settings.'); }
    } catch (e: any) { setModelsError(e.message); }
    finally { setIsLoadingModels(false); }
  }, []);
  useEffect(() => { fetchModels(); }, [fetchModels]);

  // ── Editor ─────────────────────────────────────────────────────────────────
  useEffect(() => {
    if (!editorRef.current) return;
    const inst = monaco.editor.create(editorRef.current, {
      value: `// Welcome to Keyboard\n// Open a folder (top bar), launch an agent, or start coding here.\n\nfunction helloWorld() {\n  console.log("Hello, World!");\n}\n`,
      language: 'typescript', theme: 'vs-dark', automaticLayout: true, fontSize: 14,
      fontFamily: "'Cascadia Code', 'Fira Code', 'Consolas', monospace", minimap: { enabled: true },
      scrollBeyondLastLine: false, tabSize: 2,
    });
    setEditor(inst);
    return () => inst.dispose();
  }, []);

  // ── Chat ───────────────────────────────────────────────────────────────────
  const handleSendMessage = async () => {
    const msg = inputValue.trim();
    if (!msg || !selectedModel || isGenerating) return;
    setInputValue('');
    setIsGenerating(true);
    setChatMessages(p => [...p, { role: 'user', content: msg, timestamp: new Date() }]);
    try {
      const r = await window.electronAPI.aiGenerate(selectedProvider, selectedModel, msg, { temperature });
      if (r.error) throw new Error(r.error);
      setChatMessages(p => [...p, { role: 'assistant', content: r.response || '', timestamp: new Date(), provider: selectedProvider, model: selectedModel }]);
    } catch (e: any) {
      setChatMessages(p => [...p, { role: 'assistant', content: `**Error**: ${e.message}`, timestamp: new Date(), provider: selectedProvider, model: selectedModel }]);
    } finally { setIsGenerating(false); }
  };

  // ── Agents / custom buttons → terminal ─────────────────────────────────────
  const runInTerminal = (cmd: string) => {
    setShowTerminal(true);
    // allow the pane to mount before launching
    setTimeout(() => terminalRef.current?.launchCommand(cmd), 60);
  };
  const launchAgent = (a: AgentInfo) => runInTerminal(a.command + (a.runArgs ? ` ${a.runArgs}` : ''));

  // ── Model management ───────────────────────────────────────────────────────
  const openModelManager = async () => {
    setShowModelManager(true);
    setPullStatus(null);
    try { const r = await window.electronAPI.listRegistry(registrySearch); setRegistryModels(r.models || []); } catch { /* ignore */ }
  };
  const handlePullModel = async (name: string) => {
    setIsPulling(name); setPullStatus(`Downloading ${name}… (this can take a while)`);
    try { const r = await window.electronAPI.pullModel(name); if (r.error) setPullStatus(`Error: ${r.error}`); else { setPullStatus(`${name} downloaded!`); fetchModels(); } }
    catch (e: any) { setPullStatus(`Error: ${e.message}`); } finally { setIsPulling(null); }
  };
  const handleDeleteModel = async (name: string) => {
    if (!confirm(`Delete model "${name}"?`)) return;
    setIsDeleting(name);
    try { await window.electronAPI.deleteModel(name); fetchModels(); } catch (e: any) { alert(`Delete failed: ${e.message}`); } finally { setIsDeleting(null); }
  };

  // ── Git (shell-agnostic: use return codes, not && / ||) ─────────────────────
  const fetchGitStatus = async () => {
    setGitLoading(true);
    try {
      const r = await window.electronAPI.executeCommand('git status --porcelain -b', workingDir);
      if (r.returncode !== 0 || /not a git repository/i.test(r.stderr)) { setGitStatus(null); return; }
      const lines = r.stdout.split('\n').filter(Boolean);
      const branchLine = lines[0] || '';
      const branch = branchLine.replace(/^## /, '').split('...')[0] || 'unknown';
      const ahead = branchLine.includes('ahead') ? parseInt(branchLine.match(/ahead (\d+)/)?.[1] || '0') : 0;
      const behind = branchLine.includes('behind') ? parseInt(branchLine.match(/behind (\d+)/)?.[1] || '0') : 0;
      const files = lines.slice(1).map(l => ({
        path: l.substring(3), status: l.substring(0, 2).trim() || '?',
        staged: l.charAt(0) !== ' ' && l.charAt(0) !== '?',
      }));
      setGitStatus({ branch, changes: files.filter(f => !f.staged).length, staged: files.filter(f => f.staged).length, files, ahead, behind });
    } catch { setGitStatus(null); } finally { setGitLoading(false); }
  };
  const handleGitCommit = async () => {
    if (!commitMsg.trim()) return;
    try {
      await window.electronAPI.executeCommand('git add -A', workingDir);
      const r = await window.electronAPI.executeCommand(`git commit -m "${commitMsg.replace(/"/g, '\\"')}"`, workingDir);
      if (r.returncode !== 0 && r.stderr) alert(`Commit: ${r.stderr || r.stdout}`);
      setCommitMsg(''); fetchGitStatus();
    } catch (e: any) { alert(`Commit failed: ${e.message}`); }
  };
  const handleGitDiff = async (file: string) => {
    try {
      let r = await window.electronAPI.executeCommand(`git diff -- "${file}"`, workingDir);
      if (!r.stdout.trim()) r = await window.electronAPI.executeCommand(`git diff --cached -- "${file}"`, workingDir);
      setGitDiff(r.stdout || 'No diff available');
    } catch { setGitDiff('Error fetching diff'); }
  };
  const handleGitBranch = async (action: 'create' | 'checkout', name: string) => {
    if (!name.trim()) return;
    try {
      const cmd = action === 'create' ? `git checkout -b "${name}"` : `git checkout "${name}"`;
      const r = await window.electronAPI.executeCommand(cmd, workingDir);
      if (r.returncode !== 0 && r.stderr) alert(r.stderr);
      setGitBranchInput(''); fetchGitStatus();
    } catch (e: any) { alert(`Branch failed: ${e.message}`); }
  };

  // ── Scaffold (write_file creates parent dirs itself) ───────────────────────
  const handleScaffold = async (template: ScaffoldTemplate) => {
    const base = scaffoldTarget.trim() || (workingDir ? `${workingDir}/new-project` : '');
    if (!base) { alert('Set a target directory first (or open a working folder).'); return; }
    setScaffolding(true);
    try {
      for (const f of template.files) {
        const full = base.replace(/[\\/]+$/, '') + '/' + f.path;
        const res = await window.electronAPI.backendAction('write_file', { path: full, content: f.content });
        if (res?.error) throw new Error(res.error);
      }
      alert(`Project scaffolded at ${base}`);
    } catch (e: any) { alert(`Scaffold failed: ${e.message}`); } finally { setScaffolding(false); }
  };

  // ── Files (native dialogs) ─────────────────────────────────────────────────
  const handleFileOpen = async () => {
    const r = await window.electronAPI.openFileDialog();
    if (r.canceled || r.content == null || !editor) return;
    editor.setValue(r.content);
    setActiveFile(r.name || 'untitled');
    monaco.editor.setModelLanguage(editor.getModel()!, detectLang(r.name || ''));
  };
  const handleSaveFile = async () => {
    if (!editor) return;
    await window.electronAPI.saveFileDialog(editor.getValue(), activeFile);
  };

  // ── Code actions / tests → chat prompt ─────────────────────────────────────
  const handleGenerateTests = () => {
    if (!editor) return;
    const code = editor.getValue(); if (!code.trim()) return;
    const lang = detectLang(activeFile);
    setInputValue(
      lang === 'python' ? `Generate pytest unit tests for this code. Output ONLY the test code:\n\n${code}`
        : lang === 'typescript' || lang === 'javascript' ? `Generate Jest/Vitest unit tests for this code. Output ONLY the test code:\n\n${code}`
          : `Generate unit tests for this code. Output ONLY the test code:\n\n${code}`
    );
  };
  const handleCodeAction = (action: 'refactor' | 'docs' | 'review') => {
    if (!editor) return;
    const code = editor.getValue(); if (!code.trim()) return;
    const prompts: Record<string, string> = {
      refactor: `Refactor this code for better readability and performance. Output ONLY the improved code:\n\n${code}`,
      docs: `Add comprehensive JSDoc/docstring documentation to this code. Output the documented code:\n\n${code}`,
      review: `Review this code for bugs, security issues, and best practices. Be concise:\n\n${code}`,
    };
    setInputValue(prompts[action]);
  };

  // ── Terminal vertical resize ───────────────────────────────────────────────
  const startTerminalResize = (e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY; const startH = terminalHeight;
    const onMove = (ev: MouseEvent) => setTerminalHeight(Math.max(120, Math.min(window.innerHeight - 200, startH + (startY - ev.clientY))));
    const onUp = () => { window.removeEventListener('mousemove', onMove); window.removeEventListener('mouseup', onUp); };
    window.addEventListener('mousemove', onMove); window.addEventListener('mouseup', onUp);
  };

  const pickWorkingDir = async () => {
    const r = await window.electronAPI.pickDirectory();
    if (!r.canceled && r.path) { const s = { ...settings, workingDir: r.path }; setSettings(s); window.electronAPI.setSettings(s); fetchGitStatus(); }
  };

  const installedAgents = agents.filter(a => a.installed);
  const dirLabel = workingDir ? workingDir.split(/[\\/]/).pop() : 'No folder';

  // Providers = built-ins + user-configured OpenAI-compatible ones.
  const providerList = [
    { id: 'ollama', name: 'Ollama' },
    { id: 'claude', name: 'Claude' },
    ...settings.customProviders.map(c => ({ id: c.id, name: c.name })),
  ];
  const providerName = (id: string) => PROVIDER_NAMES[id] || settings.customProviders.find(c => c.id === id)?.name || id;
  const providerColor = (id: string) => PROVIDER_COLORS[id] || '#10a37f';
  const selectProvider = (id: string) => { setSelectedProvider(id); const m = models.filter(mm => mm.provider === id); if (m.length) setSelectedModel(m[0].name); };

  // ── Render ─────────────────────────────────────────────────────────────────
  return (
    <div style={styles.container}>
      {/* Background layer */}
      {settings.background.type !== 'none' && (
        <div style={{
          position: 'fixed', inset: 0, zIndex: 0, pointerEvents: 'none',
          ...(settings.background.type === 'image' && settings.background.imageDataUrl
            ? { backgroundImage: `url(${settings.background.imageDataUrl})`, backgroundSize: 'cover', backgroundPosition: 'center' }
            : { background: settings.background.color || '#1e1e1e' }),
        }} />
      )}

      {/* Title Bar */}
      <div style={styles.titleBar}>
        <span style={styles.titleText}>⌨️ Keyboard</span>
        <span style={styles.titleCenter}>
          <button onClick={() => setSidebarTab('models')} style={{ ...styles.tabBtn, ...(sidebarTab === 'models' ? styles.tabBtnActive : {}) }}>🤖 Models</button>
          <button onClick={() => { setSidebarTab('git'); fetchGitStatus(); }} style={{ ...styles.tabBtn, ...(sidebarTab === 'git' ? styles.tabBtnActive : {}) }}>🔀 Git</button>
          <button onClick={() => setSidebarTab('scaffold')} style={{ ...styles.tabBtn, ...(sidebarTab === 'scaffold' ? styles.tabBtnActive : {}) }}>📦 New</button>
        </span>

        <span style={styles.toolbar}>
          {/* Working folder */}
          <button onClick={pickWorkingDir} style={styles.folderBtn} title={workingDir || 'Open a working folder'}>📁 {dirLabel}</button>

          {/* Detected agents */}
          {installedAgents.map(a => (
            <button key={a.id} onClick={() => launchAgent(a)} style={styles.agentBtn} title={`Run ${a.command}${a.runArgs ? ' ' + a.runArgs : ''} in terminal`}>
              {a.icon} {a.label}
            </button>
          ))}

          {/* Custom buttons */}
          {settings.customButtons.map(b => (
            <button key={b.id} onClick={() => runInTerminal(b.command)} style={styles.customBtn} title={b.command}>
              {b.icon} {b.label}
            </button>
          ))}

          <button onClick={() => setShowTerminal(v => !v)} style={{ ...styles.iconBtn, ...(showTerminal ? styles.iconBtnActive : {}) }} title="Toggle terminal (Ctrl+`)">🖥️</button>
          <button onClick={() => setShowSettings(true)} style={styles.iconBtn} title="Settings">⚙️</button>
          {!isLoadingModels && <span style={{ fontSize: 11, color: '#888', whiteSpace: 'nowrap' }}>{models.filter(m => m.provider === 'ollama').length > 0 || models.length ? '🟢' : '🟡'} {models.length}</span>}
        </span>
      </div>

      <div style={styles.mainLayout}>
        {/* ─── Sidebar ─────────────────────────────────────────────── */}
        <div style={styles.sidebar}>
          {sidebarTab === 'models' && (
            <>
              <div style={styles.section}>
                <label style={styles.label}>Provider</label>
                <div style={{ ...styles.providerGroup, flexWrap: 'wrap' }}>
                  {providerList.map(p => (
                    <button key={p.id} onClick={() => selectProvider(p.id)}
                      style={{ ...styles.providerBtn, ...(selectedProvider === p.id ? styles.providerBtnActive : {}), borderColor: providerColor(p.id), ...(selectedProvider === p.id ? { backgroundColor: providerColor(p.id) + '22' } : {}) }}>
                      {p.name}
                    </button>
                  ))}
                </div>
                {settings.customProviders.length === 0 && (
                  <div style={{ fontSize: 10, color: '#666', marginTop: 4 }}>Add OpenAI-compatible providers in ⚙️ Settings.</div>
                )}
              </div>

              <div style={styles.section}>
                <label style={styles.label}>Model</label>
                {isLoadingModels ? <div style={styles.loadingText}>Loading…</div> :
                  modelsError ? <div><div style={styles.errorText}>{modelsError}</div><button onClick={fetchModels} style={styles.retryBtn}>Retry</button></div> :
                    filteredModels.length === 0 ? <div style={styles.loadingText}>No models</div> :
                      <select value={selectedModel} onChange={e => setSelectedModel(e.target.value)} style={styles.select}>
                        {filteredModels.map(m => <option key={`${m.provider}:${m.name}`} value={m.name}>{m.name}{m.details?.parameter_size ? ` (${m.details.parameter_size})` : ''}</option>)}
                      </select>}
              </div>

              <div style={styles.section}>
                <label style={styles.label}>Temperature: {temperature.toFixed(1)}</label>
                <input type="range" min="0" max="2" step="0.1" value={temperature} onChange={e => setTemperature(parseFloat(e.target.value))} style={{ width: '100%' }} />
              </div>

              {(() => {
                const m = models.find(mm => mm.name === selectedModel);
                if (!m) return null;
                return (
                  <div style={styles.section}>
                    <label style={styles.label}>Model Details</label>
                    <div style={styles.modelInfo}>
                      <div style={styles.modelInfoRow}><span>Provider</span><span style={{ color: providerColor(m.provider) }}>{providerName(m.provider)}</span></div>
                      {m.details?.parameter_size && <div style={styles.modelInfoRow}><span>Parameters</span><span>{m.details.parameter_size}</span></div>}
                      {m.details?.family && <div style={styles.modelInfoRow}><span>Family</span><span>{m.details.family}</span></div>}
                      {m.size > 0 && <div style={styles.modelInfoRow}><span>Size</span><span>{formatSize(m.size)}</span></div>}
                      {m.provider === 'ollama' && (
                        <button onClick={() => handleDeleteModel(m.name)} disabled={isDeleting === m.name} style={styles.dangerBtn}>{isDeleting === m.name ? '⏳' : '🗑️'} Delete</button>
                      )}
                    </div>
                  </div>
                );
              })()}

              <div style={styles.section}>
                <label style={styles.label}>File</label>
                <button onClick={handleFileOpen} style={styles.actionBtn}>📂 Open File</button>
                <button onClick={handleSaveFile} style={styles.actionBtn}>💾 Save File As…</button>
              </div>
              <div style={styles.section}>
                <label style={styles.label}>Code Actions</label>
                <button onClick={() => handleCodeAction('refactor')} style={styles.actionBtn}>🔧 Refactor Code</button>
                <button onClick={() => handleCodeAction('docs')} style={styles.actionBtn}>📝 Add Docs</button>
                <button onClick={() => handleCodeAction('review')} style={styles.actionBtn}>🔍 Review Code</button>
                <button onClick={handleGenerateTests} style={styles.actionBtn}>🧪 Generate Tests</button>
              </div>
              <div style={styles.section}>
                <button onClick={() => setChatMessages([])} disabled={chatMessages.length === 0} style={styles.actionBtn}>🗑️ Clear Chat</button>
                <button onClick={openModelManager} style={styles.actionBtn}>📥 Download Models</button>
              </div>
            </>
          )}

          {sidebarTab === 'git' && (
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 4, overflow: 'hidden' }}>
              <div style={styles.section}>
                <label style={styles.label}>Repository</label>
                <div style={{ fontSize: 11, color: '#888', marginBottom: 6 }}>📁 {workingDir || 'No folder open'}</div>
                <button onClick={fetchGitStatus} style={styles.actionBtn}>{gitLoading ? '⏳ Refreshing…' : '🔄 Refresh'}</button>
              </div>
              {gitStatus ? (
                <>
                  <div style={styles.section}>
                    <div style={styles.modelInfo}>
                      <div style={styles.modelInfoRow}><span>Branch</span><span style={{ color: '#4ec9b0' }}>{gitStatus.branch}</span></div>
                      <div style={styles.modelInfoRow}><span>Modified</span><span>{gitStatus.changes} files</span></div>
                      <div style={styles.modelInfoRow}><span>Staged</span><span>{gitStatus.staged} files</span></div>
                      {gitStatus.ahead ? <div style={styles.modelInfoRow}><span>Ahead</span><span>{gitStatus.ahead}</span></div> : null}
                      {gitStatus.behind ? <div style={styles.modelInfoRow}><span>Behind</span><span>{gitStatus.behind}</span></div> : null}
                    </div>
                  </div>
                  <div style={styles.section}>
                    <label style={styles.label}>Branch</label>
                    <input type="text" value={gitBranchInput} onChange={e => setGitBranchInput(e.target.value)} placeholder="branch name" style={{ ...styles.chatInput, fontSize: 12, padding: '4px 6px' }} />
                    <div style={{ display: 'flex', gap: 4, marginTop: 4 }}>
                      <button onClick={() => handleGitBranch('checkout', gitBranchInput)} style={{ ...styles.actionBtn, flex: 1, fontSize: 11 }}>🔀 Switch</button>
                      <button onClick={() => handleGitBranch('create', gitBranchInput)} style={{ ...styles.actionBtn, flex: 1, fontSize: 11 }}>➕ New</button>
                    </div>
                  </div>
                  <div style={{ ...styles.section, flex: 1, overflow: 'auto', minHeight: 80 }}>
                    <label style={styles.label}>Changed Files</label>
                    {gitStatus.files.length === 0 ? <div style={styles.loadingText}>Clean working tree</div> :
                      gitStatus.files.map((f, i) => (
                        <div key={i} onClick={() => handleGitDiff(f.path)} style={{ ...styles.fileRow, cursor: 'pointer', borderLeft: `3px solid ${f.staged ? '#4ec9b0' : '#e06c75'}` }}>
                          <span style={{ fontSize: 11, color: f.staged ? '#4ec9b0' : '#e06c75', width: 20 }}>{f.status}</span>
                          <span style={{ fontSize: 11, flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{f.path}</span>
                        </div>
                      ))}
                  </div>
                  <div style={styles.section}>
                    <label style={styles.label}>Commit</label>
                    <input type="text" value={commitMsg} onChange={e => setCommitMsg(e.target.value)} placeholder="Commit message" style={{ ...styles.chatInput, fontSize: 12, padding: '4px 6px', marginBottom: 4 }} onKeyDown={e => { if (e.key === 'Enter') handleGitCommit(); }} />
                    <button onClick={handleGitCommit} disabled={!commitMsg.trim()} style={styles.actionBtn}>✅ Commit (add all + commit)</button>
                  </div>
                  {gitDiff && (
                    <div style={styles.section}>
                      <label style={styles.label}>Diff <span onClick={() => setGitDiff('')} style={{ cursor: 'pointer', color: '#e06c75' }}>×</span></label>
                      <pre style={{ fontSize: 10, maxHeight: 120, overflow: 'auto', backgroundColor: '#1e1e1e', padding: 4, borderRadius: 4, color: '#d4d4d4' }}>{gitDiff}</pre>
                    </div>
                  )}
                </>
              ) : (
                <div style={{ padding: '12px 0', color: '#888', fontSize: 12, textAlign: 'center' }}>
                  {gitLoading ? 'Checking…' : workingDir ? 'Not a git repository' : 'Open a working folder (top bar)'}
                </div>
              )}
            </div>
          )}

          {sidebarTab === 'scaffold' && (
            <div style={{ flex: 1 }}>
              <label style={styles.label}>Project Templates</label>
              <input type="text" value={scaffoldTarget} onChange={e => setScaffoldTarget(e.target.value)} placeholder={workingDir ? `${workingDir}/my-project` : 'Target directory'} style={{ ...styles.chatInput, fontSize: 12, padding: '4px 6px', margin: '4px 0 8px' }} />
              {SCAFFOLD_TEMPLATES.map((t, i) => (
                <div key={i} style={styles.templateCard}>
                  <div style={{ fontWeight: 600, fontSize: 12, marginBottom: 2 }}>{t.name}</div>
                  <div style={{ fontSize: 11, color: '#888', marginBottom: 6 }}>{t.description}</div>
                  <div style={{ fontSize: 10, color: '#666', marginBottom: 6 }}>{t.files.length} files · {t.language}</div>
                  <button onClick={() => handleScaffold(t)} disabled={scaffolding} style={styles.actionBtn}>{scaffolding ? '⏳ Creating…' : '📦 Generate'}</button>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* ─── Content (editor + chat) ──────────────────────────────── */}
        <div style={styles.contentArea}>
          <div style={styles.editorPanel}>
            <div style={styles.panelHeader}><span>📝 {activeFile}</span><span style={{ fontSize: 11, color: '#888' }}>{detectLang(activeFile)}</span></div>
            <div ref={editorRef} style={{ width: '100%', flex: 1, minHeight: 0 }} />
          </div>
          <div style={styles.divider} />
          <div style={styles.chatPanel}>
            <div style={styles.panelHeader}><span>💬 Chat</span><span style={{ fontSize: 11, color: '#888' }}>{isGenerating ? 'Generating…' : `${chatMessages.length} msgs`}</span></div>
            <div style={styles.messagesContainer}>
              {chatMessages.length === 0 && !isGenerating && (
                <div style={styles.emptyState}><div style={{ fontSize: 32, marginBottom: 4 }}>💬</div><div>Ask the AI for help</div><div style={{ fontSize: 11, color: '#666', marginTop: 2 }}>{selectedModel || 'No model selected'}</div></div>
              )}
              {chatMessages.map((msg, i) => (
                <div key={i} style={{ ...styles.messageBubble, ...(msg.role === 'user' ? styles.userBubble : styles.aiBubble) }}>
                  <div style={styles.messageHeader}>
                    <strong>{msg.role === 'user' ? 'You' : 'AI'}</strong>
                    <span style={styles.messageMeta}>{msg.provider && <span style={{ ...styles.providerTag, color: providerColor(msg.provider) }}>{providerName(msg.provider)}</span>}{formatTime(msg.timestamp)}</span>
                  </div>
                  <div style={styles.messageContent}>{msg.content}</div>
                </div>
              ))}
              {isGenerating && (
                <div style={{ ...styles.messageBubble, ...styles.aiBubble }}>
                  <div style={styles.messageHeader}><strong>AI</strong><span style={styles.messageMeta}><span style={{ ...styles.providerTag, color: providerColor(selectedProvider) }}>{providerName(selectedProvider)}</span>⏳</span></div>
                  <div style={styles.messageContent}><span style={styles.typingIndicator}>▊</span></div>
                </div>
              )}
              <div ref={chatEndRef} />
            </div>
            <div style={styles.inputArea}>
              <input type="text" value={inputValue} onChange={e => setInputValue(e.target.value)} onKeyDown={e => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSendMessage(); } }}
                placeholder={selectedModel ? 'Type a message… (Enter to send)' : 'Select a model'} disabled={isGenerating || !selectedModel} style={styles.chatInput} />
              <button onClick={handleSendMessage} disabled={isGenerating || !inputValue.trim() || !selectedModel} style={styles.sendButton}>{isGenerating ? '⏳' : 'Send'}</button>
            </div>
          </div>
        </div>
      </div>

      {/* ─── Terminal dock ────────────────────────────────────────── */}
      {showTerminal && (
        <div style={{ ...styles.terminalDock, height: terminalHeight }}>
          <div style={styles.terminalResizeHandle} onMouseDown={startTerminalResize} />
          <div style={styles.terminalDockHeader}>
            <span style={{ fontSize: 11, color: '#888' }}>TERMINAL · {ptyMode === 'piped' ? 'piped shell' : 'PowerShell (PTY)'}</span>
            <button onClick={() => setShowTerminal(false)} style={styles.iconBtn} title="Hide terminal">▽</button>
          </div>
          <div style={{ flex: 1, minHeight: 0 }}>
            <TerminalPane ref={terminalRef} cwd={workingDir} fontSize={settings.terminalFontSize} accent={settings.accentColor} />
          </div>
        </div>
      )}

      {/* ─── Modals ───────────────────────────────────────────────── */}
      {showModelManager && (
        <div style={styles.modalOverlay} onClick={() => setShowModelManager(false)}>
          <div style={styles.modal} onClick={e => e.stopPropagation()}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
              <h3 style={{ margin: 0, color: '#d4d4d4' }}>📥 Download Ollama Models</h3>
              <button onClick={() => setShowModelManager(false)} style={{ background: 'none', border: 'none', color: '#888', fontSize: 18, cursor: 'pointer' }}>×</button>
            </div>
            <input type="text" value={registrySearch} onChange={e => setRegistrySearch(e.target.value)} placeholder="Search models…" style={{ ...styles.chatInput, marginBottom: 8, fontSize: 12 }}
              onKeyDown={async e => { if (e.key === 'Enter') { try { const r = await window.electronAPI.listRegistry(registrySearch); setRegistryModels(r.models || []); } catch { /* ignore */ } } }} />
            <div style={{ maxHeight: 300, overflowY: 'auto' }}>
              {registryModels.length === 0 ? <div style={{ color: '#888', fontSize: 12, padding: 8 }}>No models. Type to search.</div> :
                registryModels.map((m, i) => (
                  <div key={i} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '6px 8px', borderBottom: '1px solid #3c3c3c' }}>
                    <div><span style={{ fontSize: 12, color: '#d4d4d4' }}>{m.name}</span>{m.installed && <span style={{ fontSize: 10, color: '#4ec9b0', marginLeft: 6 }}>✔️ installed</span>}</div>
                    {!m.installed && <button onClick={() => handlePullModel(m.name)} disabled={isPulling === m.name} style={styles.actionBtn}>{isPulling === m.name ? '⏳' : '📥'} Download</button>}
                  </div>
                ))}
            </div>
            {pullStatus && <div style={{ fontSize: 12, color: '#888', padding: 8 }}>{pullStatus}</div>}
          </div>
        </div>
      )}

      {showSettings && <SettingsModal settings={settings} onClose={() => setShowSettings(false)} onSave={saveSettings} />}
    </div>
  );
};

// ─── Styles ──────────────────────────────────────────────────────────────────
const styles: Record<string, React.CSSProperties> = {
  container: { position: 'relative', zIndex: 1, display: 'flex', flexDirection: 'column', height: '100vh', fontFamily: "'Segoe UI', -apple-system, BlinkMacSystemFont, sans-serif", backgroundColor: 'var(--bg-base)', color: '#d4d4d4' },
  titleBar: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '6px 12px', backgroundColor: 'var(--bg-elevated)', borderBottom: '1px solid #3c3c3c', userSelect: 'none', fontSize: 13, gap: 8 },
  titleText: { fontWeight: 600, color: '#e0e0e0', whiteSpace: 'nowrap' },
  titleCenter: { display: 'flex', gap: 2 },
  tabBtn: { background: 'none', border: 'none', color: '#888', cursor: 'pointer', fontSize: 12, padding: '4px 10px', borderRadius: 4 },
  tabBtnActive: { backgroundColor: '#3c3c3c', color: '#d4d4d4' },
  toolbar: { display: 'flex', alignItems: 'center', gap: 6, overflowX: 'auto', flex: 1, justifyContent: 'flex-end' },
  folderBtn: { background: '#1e1e1e', border: '1px solid #3c3c3c', color: '#ccc', cursor: 'pointer', fontSize: 11, padding: '4px 8px', borderRadius: 4, whiteSpace: 'nowrap', maxWidth: 160, overflow: 'hidden', textOverflow: 'ellipsis' },
  agentBtn: { background: 'var(--accent)', border: 'none', color: '#fff', cursor: 'pointer', fontSize: 11, fontWeight: 600, padding: '4px 10px', borderRadius: 4, whiteSpace: 'nowrap' },
  customBtn: { background: '#333', border: '1px solid #4c4c4c', color: '#e0e0e0', cursor: 'pointer', fontSize: 11, padding: '4px 10px', borderRadius: 4, whiteSpace: 'nowrap' },
  iconBtn: { background: 'none', border: '1px solid transparent', color: '#aaa', cursor: 'pointer', fontSize: 14, padding: '2px 6px', borderRadius: 4 },
  iconBtnActive: { background: '#3c3c3c', color: '#fff' },
  mainLayout: { display: 'flex', flex: 1, overflow: 'hidden', minHeight: 0 },
  sidebar: { width: 240, backgroundColor: 'var(--bg-panel)', borderRight: '1px solid #3c3c3c', padding: 12, display: 'flex', flexDirection: 'column', gap: 4, overflowY: 'auto', flexShrink: 0 },
  section: { marginBottom: 12 },
  label: { display: 'block', fontSize: 11, fontWeight: 600, textTransform: 'uppercase', color: '#888', marginBottom: 6, letterSpacing: '0.5px' },
  providerGroup: { display: 'flex', gap: 4 },
  providerBtn: { flex: 1, padding: '6px 8px', fontSize: 12, fontWeight: 500, cursor: 'pointer', backgroundColor: '#1e1e1e', color: '#ccc', border: '1px solid #3c3c3c', borderRadius: 4 },
  providerBtnActive: { backgroundColor: '#2d2d2d', color: '#fff' },
  select: { width: '100%', padding: '6px 8px', fontSize: 12, backgroundColor: '#1e1e1e', color: '#d4d4d4', border: '1px solid #3c3c3c', borderRadius: 4, outline: 'none' },
  loadingText: { fontSize: 12, color: '#888', padding: '4px 0' },
  errorText: { fontSize: 12, color: '#e06c75', padding: '4px 0', marginBottom: 4 },
  retryBtn: { fontSize: 11, padding: '3px 8px', cursor: 'pointer', backgroundColor: '#2d2d2d', color: '#d4d4d4', border: '1px solid #3c3c3c', borderRadius: 3 },
  modelInfo: { fontSize: 11, backgroundColor: '#1e1e1e', border: '1px solid #3c3c3c', borderRadius: 4, padding: '6px 8px' },
  modelInfoRow: { display: 'flex', justifyContent: 'space-between', padding: '2px 0', color: '#999' },
  actionBtn: { width: '100%', padding: '6px 8px', fontSize: 12, cursor: 'pointer', backgroundColor: '#1e1e1e', color: '#d4d4d4', border: '1px solid #3c3c3c', borderRadius: 4, textAlign: 'left', marginBottom: 4 },
  dangerBtn: { width: '100%', padding: '6px 8px', fontSize: 12, cursor: 'pointer', backgroundColor: '#3e1a1a', color: '#e06c75', border: '1px solid #6e2a2a', borderRadius: 4, textAlign: 'center', marginTop: 4 },
  fileRow: { display: 'flex', alignItems: 'center', gap: 4, padding: '3px 4px', borderBottom: '1px solid #2d2d2d' },
  contentArea: { flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden', minWidth: 0 },
  editorPanel: { flex: 1, display: 'flex', flexDirection: 'column', minHeight: 120 },
  panelHeader: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '6px 12px', backgroundColor: 'var(--bg-elevated)', borderBottom: '1px solid #3c3c3c', fontSize: 12, color: '#999', userSelect: 'none' },
  divider: { height: 3, backgroundColor: '#2d2d2d' },
  chatPanel: { flex: 1, display: 'flex', flexDirection: 'column', minHeight: 120, maxHeight: '50%' },
  messagesContainer: { flex: 1, overflowY: 'auto', padding: 12, display: 'flex', flexDirection: 'column', gap: 8 },
  emptyState: { display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', height: '100%', color: '#666', fontSize: 13 },
  messageBubble: { padding: '10px 12px', borderRadius: 8, maxWidth: '85%', fontSize: 13, lineHeight: 1.5 },
  userBubble: { backgroundColor: '#264f78', alignSelf: 'flex-end', borderBottomRightRadius: 4 },
  aiBubble: { backgroundColor: '#2d2d2d', alignSelf: 'flex-start', borderBottomLeftRadius: 4, border: '1px solid #3c3c3c' },
  messageHeader: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4, fontSize: 11, color: '#888' },
  messageMeta: { display: 'flex', alignItems: 'center', gap: 6 },
  providerTag: { fontSize: 10, fontWeight: 600, textTransform: 'uppercase' },
  messageContent: { whiteSpace: 'pre-wrap', wordBreak: 'break-word' },
  typingIndicator: { animation: 'blink 1s step-end infinite' },
  inputArea: { display: 'flex', gap: 8, padding: '10px 12px', borderTop: '1px solid #3c3c3c', backgroundColor: 'var(--bg-panel)' },
  chatInput: { flex: 1, padding: '8px 12px', fontSize: 13, backgroundColor: '#1e1e1e', color: '#d4d4d4', border: '1px solid #3c3c3c', borderRadius: 6, outline: 'none' },
  sendButton: { padding: '8px 16px', fontSize: 13, fontWeight: 600, cursor: 'pointer', backgroundColor: 'var(--accent)', color: '#fff', border: 'none', borderRadius: 6, minWidth: 60 },
  terminalDock: { display: 'flex', flexDirection: 'column', borderTop: '1px solid #3c3c3c', backgroundColor: '#1a1a1a', position: 'relative', flexShrink: 0 },
  terminalResizeHandle: { position: 'absolute', top: -3, left: 0, right: 0, height: 6, cursor: 'row-resize', zIndex: 5 },
  terminalDockHeader: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '2px 10px', backgroundColor: 'var(--bg-elevated)', borderBottom: '1px solid #3c3c3c' },
  modalOverlay: { position: 'fixed', inset: 0, backgroundColor: 'rgba(0,0,0,0.6)', display: 'flex', justifyContent: 'center', alignItems: 'center', zIndex: 1000 },
  modal: { backgroundColor: '#252526', border: '1px solid #3c3c3c', borderRadius: 8, padding: 16, width: 450, maxHeight: '70vh', display: 'flex', flexDirection: 'column' },
  templateCard: { backgroundColor: '#1e1e1e', border: '1px solid #3c3c3c', borderRadius: 6, padding: '10px 12px', marginBottom: 8 },
};

export default App;
