# T.C.K — Feature Status & Roadmap

T.C.K (TalentCloud Keyboard) is a **100% Rust** rewrite of the original
Electron/TypeScript app. The UI is Rust → WebAssembly (Dioxus); the backend is
Rust (axum). This document tracks which capabilities are implemented in the Rust
version and what's planned next.

Legend: ✅ done · 🟡 partial · ⬜ planned

## Implemented in the Rust rewrite

- ✅ **Runs in the browser** — entire UI compiled to WebAssembly.
- ✅ **Real terminal** — PowerShell/bash via `portable-pty`, streamed to the browser over a WebSocket (`crates/tck-server`), rendered in `crates/tck-ui`.
- ✅ **Ollama integration** — `/api/generate` proxy with graceful error reporting.
- ✅ **Claude integration** — Anthropic Messages API via the server proxy.
- ✅ **OpenAI-compatible providers** — OpenAI, OpenRouter, Groq, DeepSeek, Mistral, OpenCode Zen, LM Studio, llama.cpp (presets in `tck-core`).
- ✅ **AI agent launchers** — Claude Code, Codex, OpenCode, Gemini CLI, Aider buttons that run in the terminal.
- ✅ **Custom command buttons** — user-defined commands run in the terminal.
- ✅ **Settings** persisted to the browser's `localStorage`.
- ✅ **Code editor** scratch buffer.

## Planned / partial

- 🟡 **Terminal rendering** — ANSI escape codes are stripped for legibility; a
  full VT/ANSI emulator (colors, cursor addressing) is not yet implemented.
- ⬜ **Syntax highlighting** in the editor (e.g. via a Rust highlighter compiled to WASM).
- ⬜ **File tree / open real files** (needs a file-access API in the server).
- ⬜ **Model discovery UI** (list `/v1/models` from a provider in-app).
- ⬜ **Streaming AI responses** (token-by-token over SSE/WebSocket).
- ⬜ **Multiple terminal tabs.**
- ⬜ **Themes / custom background.**
- ⬜ **Desktop packaging** (e.g. via Dioxus desktop or Tauri) alongside the browser build.
