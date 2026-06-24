# Enhancements — Implementation Status

Originally extracted from README.md as a roadmap. As of v2.0 the backend was
ported to native Node (no Python required) and most items below are implemented.

Legend: ✅ done · 🟡 partial · ⬜ not yet

## Real Ollama API Integration ✅
- ✅ Real API calls to Ollama (no placeholders) — `main.ts` calls `/api/generate`
- ✅ Model discovery via `http://localhost:11434/api/tags`
- ✅ Model download (`/api/pull`) and deletion (`/api/delete`) from the UI
- 🟡 Multiple Ollama instances — single host configurable via `OLLAMA_HOST` env

## Terminal Command Execution ✅
- ✅ Real shell execution — embedded **PowerShell PTY** (node-pty + xterm.js)
- ✅ Full command history / line editing — native shell handles it inside the PTY
- ✅ Common dev commands (git, npm, agents, …) run in the real shell
- ✅ Real-time streaming output — PTY streams bytes live to the terminal pane

## Model Management UI ✅
- ✅ Model selection with metadata (size, parameters, family)
- ✅ Download / install interface (registry search + pull)
- ✅ Model deletion
- 🟡 Parameter tuning — temperature slider exposed (more params available in backend)

## Code Generation Features ✅
- ✅ Refactor / document / review actions on the editor buffer
- ✅ Test generation (pytest / Jest-Vitest aware)
- 🟡 Inline completion — available via launching a CLI agent (Claude Code, etc.)

## Git Integration ✅
- ✅ Status, changed-file list, per-file diff
- ✅ Commit (add all + commit), branch create / switch
- ✅ Operates on the selected working folder

## Project Scaffolding ✅
- ✅ Templates: React+TS, Python CLI, Node/Express, Rust CLI
- ✅ File generation (creates parent dirs automatically)

## Additional Enhancements
- ✅ AI-powered code review (Code Actions → Review)
- ✅ Test generation
- ✅ Cross-platform shells (PowerShell on Windows, bash elsewhere)
- ✅ AI coding-agent auto-detection + one-click launch into the terminal
- ✅ Custom command buttons + Settings window
- ✅ Custom background image/color, accent color, theming
- ✅ Self-contained packaging: portable .exe + NSIS installer (no Python needed)

## Still open / future
- ⬜ Streaming token-by-token rendering in the chat panel
- ⬜ Multi-file project tree / explorer in the editor
- ⬜ Plugin system
- ⬜ Multiple simultaneous Ollama hosts
