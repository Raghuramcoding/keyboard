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
- 🌐 **Runs in the browser** — the entire editor UI is WebAssembly. Works on **Windows, macOS, Linux**, and **iPhone/iPad (iOS Safari)**.
- 📱 **Responsive** — a 3-column desktop layout that collapses to a touch-friendly tabbed layout (Agents · Editor · Terminal · AI) on phones.
- 💻 **Real terminal** — a genuine shell session via a pseudo-terminal (`portable-pty`): PowerShell on Windows, your `$SHELL` (zsh/bash) on macOS & Linux, streamed to the browser over a WebSocket.
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

## Platforms

### macOS / Linux

Build and run exactly as above — `tck-server` is cross-platform. The terminal
uses your `$SHELL` (zsh on macOS by default, falling back to `/bin/bash`).

### iPhone / iPad (and any phone)

The UI is responsive and works in **iOS Safari** (and Android Chrome). Since a
phone can't run the native server itself, point it at a T.C.K server running on
your Mac/PC on the **same Wi-Fi**:

```bash
# On your computer: bind to all interfaces so the phone can reach it
TCK_ADDR=0.0.0.0:3000 cargo run -p tck-server --release
```

Then on the phone open `http://<your-computer-LAN-IP>:3000` (e.g.
`http://192.168.1.20:3000`). Find the IP with `ipconfig` (Windows),
`ipconfig getifaddr en0` (macOS), or `hostname -I` (Linux).

On a phone the layout collapses to a bottom tab bar — **Agents · Editor ·
Terminal · AI** — so each panel gets the full screen; the on-screen keyboard
drives the terminal and AI chat.

> Use `http://` on the LAN (not `https://`) so the terminal's `ws://` WebSocket
> connects. The server has no authentication — only run it on a trusted network,
> since anyone who can reach it gets shell access.

## License

MIT — see [LICENSE](LICENSE).
