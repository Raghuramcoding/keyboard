# Claude Code v2.0 — AI-Powered Coding Environment

A cross-platform desktop coding environment with a Monaco editor, an embedded
**real PowerShell terminal**, one-click **AI coding-agent launching**, dual AI
chat providers (Ollama + Anthropic Claude), and Git/scaffolding tools.

![Platform](https://img.shields.io/badge/platform-Windows%20|%20macOS%20|%20Linux-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Version](https://img.shields.io/badge/version-2.0.0-purple)

## Highlights

- **Embedded terminal (like VS Code)** — a true interactive PTY (PowerShell on
  Windows) docked at the bottom, with tabs, resize, colors, and full-screen TUIs.
- **AI agent auto-detection** — on startup the app scans your `PATH` for installed
  coding agents (Claude Code, Codex, Gemini CLI, Aider, OpenCode, Cursor Agent,
  Copilot CLI, Ollama). Each detected agent gets a toolbar button; clicking it
  opens the terminal and runs it automatically.
- **Custom command buttons** — define your own toolbar buttons (e.g. `npm run dev`)
  in the Settings window; one click runs them in the terminal.
- **Custom background & theming** — set a background image or color with adjustable
  panel transparency, plus an accent color.
- **AI chat** — Ollama (local), Claude (cloud), and **any OpenAI-compatible API**
  with provider/model switching, plus refactor / docs / review / test-generation
  actions on your editor buffer.
- **Connect any OpenAI-compatible API** — add providers (OpenAI, OpenRouter, Groq,
  Together, DeepSeek, Mistral, local LM Studio / llama.cpp, …) in Settings with a
  base URL + API key. Generation goes through the **[Vercel AI SDK](https://ai-sdk.dev)**
  (`@ai-sdk/openai-compatible`); discover/browse models from
  **[models.dev](https://models.dev)** or fetch them live from the endpoint's
  `/v1/models`.
- **Git tools** — status, diffs, commit, branch create/switch on a chosen folder.
- **Project scaffolding** — React+TS, Python CLI, Node/Express, and Rust templates.
- **Self-contained** — no Python runtime required; the backend runs natively in
  Electron's main process. Ships as a portable .exe and an NSIS installer.

## Quick Start (from source)

Only **Node.js 18+** is required to run from source.

```bash
npm install
npm run build
npm start
```

Or use the convenience launcher: `start-app.bat` (Windows), `start-app.ps1`,
or `./start-app.sh`.

Dev mode with auto-rebuild on changes:

```bash
npm run dev
```

### Optional providers / agents

- **Ollama** (local chat models): install from [ollama.ai](https://ollama.ai),
  `ollama pull qwen2.5-coder`. Override the host with the `OLLAMA_HOST` env var.
- **Claude** (cloud chat models): add your Anthropic API key in **Settings**
  (or set `ANTHROPIC_API_KEY`).
- **OpenAI-compatible** (OpenAI, OpenRouter, Groq, local LM Studio, …): in
  **Settings → AI Providers**, add a provider with its base URL + API key, then
  add models manually, fetch them from the endpoint, or browse models.dev.
- **Coding agents**: install any CLI agent (e.g. `npm i -g @anthropic-ai/claude-code`)
  and it appears as a toolbar button automatically.

## Build installers

```bash
npm run dist:win     # Windows: NSIS installer + portable .exe  -> release/
npm run dist:mac     # macOS:   DMG + ZIP
npm run dist:linux   # Linux:   AppImage + DEB
npm run package      # Unpacked app (release/win-unpacked/) — fastest
```

Windows output in `release/`:
- `Claude Code-Setup-2.0.0.exe` — installer (desktop + start-menu shortcuts,
  choose install directory). Bundles everything; no extra dependencies to install.
- `Claude Code-2.0.0-Portable.exe` — single-file runnable executable.

> The native terminal uses `node-pty`, shipped via ABI-stable N-API prebuilt
> binaries, so packaging needs **no C++ compiler** (`npmRebuild` is disabled).

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                        Electron App                          │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │
│  │ Sidebar  │  │  Monaco  │  │   Chat   │  │  Terminal     │  │
│  │ models / │  │  Editor  │  │  panel   │  │  (xterm.js +  │  │
│  │ git /    │  │          │  │          │  │   node-pty)   │  │
│  │ scaffold │  │          │  │          │  │  agent launch │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └──────┬───────┘  │
│       └─────────────┴── preload (contextBridge) ──┘          │
│                            │ IPC                              │
│  ┌─────────────────────────┴──────────────────────────────┐  │
│  │              main.ts  (native Node backend)             │  │
│  │  Ollama proxy · Claude API (fetch) · exec · file I/O    │  │
│  │  PTY mgmt · agent detection · settings · dialogs        │  │
│  └─────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
        │ http://localhost:11434         │ https://api.anthropic.com
     Ollama (optional, local)         Claude API (optional, cloud)
```

| Component | Technology |
|-----------|-----------|
| UI | Electron + React + TypeScript + Monaco |
| Terminal | xterm.js + node-pty (real PTY) |
| Backend | Native Node in Electron main (no Python) |
| AI providers | Ollama, Claude (fetch), OpenAI-compatible via Vercel AI SDK |
| Model catalog | models.dev |
| Bundler | esbuild (renderer + Monaco workers + main/preload) |
| Packaging | electron-builder |

## Build pipeline

`npm run build` runs [`build.js`](build.js), which uses esbuild to bundle:
- `src/main.ts` → `dist/main.js` (Node/CJS, `electron` + `node-pty` external)
- `src/preload.ts` → `dist/preload.js`
- `src/renderer/index.tsx` → `dist/renderer/index.js` (browser IIFE; React + Monaco)
- Monaco's 5 web workers → `dist/renderer/*.worker.js`

> **Why a bundler?** The renderer runs sandboxed (`contextIsolation: true`,
> `nodeIntegration: false`), so raw `require()` doesn't exist there. Bundling the
> renderer to a browser IIFE is what fixes the original "black screen" — the old
> `tsc`-only setup emitted CommonJS the renderer couldn't load.

A headless smoke test is built in: `CLAUDE_CODE_SMOKE=1 npx electron .` renders the
real UI, checks Monaco + xterm mounted, and runs a live PTY round-trip.

## Usage

- **Open a working folder** — top-bar 📁 button. Git and the terminal operate there.
- **Launch an agent** — click a detected agent button; the terminal opens and runs it.
- **Settings (⚙)** — custom buttons, background, accent color, terminal font size,
  Anthropic API key, working directory.
- **Toggle terminal** — the 🖥️ button.

## License

MIT
