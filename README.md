# ⌨️ T.C.K — TalentCloud Keyboard

[![CI](https://github.com/Raghuramcoding/keyboard/actions/workflows/ci.yml/badge.svg)](https://github.com/Raghuramcoding/keyboard/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**T.C.K (TalentCloud Keyboard)** is an AI-powered coding environment written
**entirely in Rust** that runs **in your browser**. The UI compiles to
WebAssembly (Dioxus); a small native server provides a real shell terminal and
proxies AI requests.

> Previously an Electron/TypeScript app, T.C.K has been fully rewritten in Rust —
> no JavaScript or TypeScript source remains. The UI is Rust → WASM, the backend
> is Rust (axum + tokio).

## Features

- 🦀 **100% Rust** — `tck-ui` (Dioxus/WASM frontend), `tck-server` (axum backend), `tck-core` (shared types).
- 🌐 **Runs in the browser** — the entire editor UI is WebAssembly.
- 💻 **Real terminal** — a genuine PowerShell/bash session via a pseudo-terminal (`portable-pty`), streamed to the browser over a WebSocket.
- 🤖 **Multi-provider AI** — Ollama, Claude, and any OpenAI-compatible endpoint (OpenAI, OpenRouter, Groq, DeepSeek, Mistral, **OpenCode Zen**, LM Studio, …).
- 🚀 **AI agent launchers** — one-click buttons that start Claude Code, Codex, OpenCode, Gemini CLI, or Aider in the terminal.
- ⚙️ **Settings** — manage providers (with presets), API keys, and custom command buttons; persisted in the browser's `localStorage`.
- 📝 **Code editor** with a scratch buffer.

## Architecture

```
┌─────────────────────────────────────────────┐
│  Browser                                     │
│  ┌───────────────────────────────────────┐  │
│  │ tck-ui  (Rust → WebAssembly, Dioxus)  │  │
│  └───────────────┬───────────────────────┘  │
└──────────────────┼──────────────────────────┘
       /api/generate│  /ws/pty (WebSocket)
┌──────────────────▼──────────────────────────┐
│  tck-server (Rust, axum + tokio)             │
│   • AI proxy  → Ollama / Claude / OpenAI-compat│
│   • PTY bridge → PowerShell / bash            │
│   • serves the WASM bundle                    │
└─────────────────────────────────────────────┘
```

| Crate        | Target                   | Responsibility                       |
|--------------|--------------------------|--------------------------------------|
| `tck-core`   | both                     | Shared serde types, provider presets |
| `tck-ui`     | `wasm32-unknown-unknown` | Dioxus UI compiled to WebAssembly    |
| `tck-server` | native                   | axum server: AI proxy + PTY + static |

## Quick start

### Prerequisites

- [Rust](https://rustup.rs) (stable)
- The WASM target: `rustup target add wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev): `cargo install trunk` (or grab a prebuilt binary)

### Build & run

```bash
# 1. Build the WebAssembly UI
cd crates/tck-ui
trunk build --release
cd ../..

# 2. Run the server (serves the UI + provides terminal/AI)
cargo run -p tck-server --release

# 3. Open the app
#    http://127.0.0.1:3000
```

### Live-reload development

```bash
# Terminal 1 — the backend
cargo run -p tck-server

# Terminal 2 — the UI with hot reload (proxies /api and /ws to :3000)
cd crates/tck-ui
trunk serve
# open http://127.0.0.1:8080
```

## Configuration

| Variable    | Default                  | Meaning                                  |
|-------------|--------------------------|------------------------------------------|
| `TCK_ADDR`  | `127.0.0.1:3000`         | Address the server binds to              |
| `TCK_DIST`  | `crates/tck-ui/dist`     | Directory of the built WASM bundle       |

Providers, API keys, and custom commands are configured in-app via **⚙ Settings**
and stored in the browser.

## License

MIT — see [LICENSE](LICENSE).
