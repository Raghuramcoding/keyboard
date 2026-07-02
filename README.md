# ⌨️ T.C.K — TalentCloud Keyboard

[![CI](https://github.com/Raghuramcoding/keyboard/actions/workflows/ci.yml/badge.svg)](https://github.com/Raghuramcoding/keyboard/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/built%20with-Rust-orange?logo=rust)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**T.C.K (TalentCloud Keyboard)** is an AI-powered coding environment that runs
**in your browser** (or as a native desktop app). The UI compiles to WebAssembly
(Dioxus); a native server provides real shell terminals, and a TypeScript AI
service (`tck-ai`) uses the Vercel AI SDK for unified provider access.

## Features

- 🎛️ **Multi-instance dashboard** — launch, switch between, and orchestrate
  multiple coding tools at once from a single browser tab. Start Claude Code,
  Codex, Aider, Gemini CLI, or OpenCode by clicking a button — each gets its
  own PTY session that keeps running even if you close the tab.
- 💬 **AI chat instances** — create standalone AI chat sessions alongside your
  terminal tools. Chat histories persist across page reloads via `localStorage`.
- 🦀 **Rust core** — `tck-ui` (Dioxus/WASM frontend), `tck-server` (axum
  backend), `tck-core` (shared types). AI service (`tck-ai`) uses **TypeScript +
  Vercel AI SDK**.
- 🌐 **Browser or desktop** — run as a web app in any browser, or install as a
  native desktop app (WebView2 on Windows, WebKit on macOS/Linux).
- 📱 **Responsive** — a 3-column desktop layout that collapses to a touch-friendly
  tabbed layout on phones.
- 💻 **Real terminals** — each tool instance gets its own genuine shell session
  via a pseudo-terminal (`portable-pty`): PowerShell on Windows, `$SHELL`
  (zsh/bash) on macOS & Linux, streamed to the browser over WebSocket with
  reconnection support (64 KB ring buffer per session).
- 🤖 **Multi-provider AI** — Ollama, Claude, and any OpenAI-compatible endpoint
  (OpenAI, OpenRouter, Groq, DeepSeek, Mistral, OpenCode Zen, LM Studio, …).
- 🐙 **Git via GitHub OAuth** — browse repos, open files into the editor, commit
  changes back.
- 🏗️ **Scaffolding** — generate starter projects (Rust CLI, static web, Node,
  Python) or push a whole template to a new GitHub repo.
- ⚙️ **Settings** — manage providers, API keys, and custom commands; persisted
  in `localStorage`.
- 📝 **Code editor** with a scratch buffer.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Browser (Dioxus WASM)         ┌──────────────────┐ │
│  ┌─────────────────────────┐   │  Desktop App      │ │
│  │  Dashboard View          │   │  (wry webview)   │ │
│  │  ┌─── ─── ─── ─── ┐    │   └────────┬─────────┘ │
│  │  │ Sidebar │ Panel │    │            │           │
│  │  │ Claude  │ ● $   │    │            │           │
│  │  │ Codex   │ input │    │            │           │
│  │  │ Aider   │       │    │            │           │
│  │  │ AI Chat │       │    │            │           │
│  │  │ + New   │       │    │            │           │
│  │  └─── ─── ─── ─── ┘    │            │           │
│  └───────────┬─────────────┘            │           │
└──────────────┼──────────────────────────┼───────────┘
       /api/instances*        /ws/pty/{id}│
┌──────────────▼──────────────────────────▼───────────┐
│ tck-server (Rust, axum + tokio)                      │
│ • InstanceManager — multi-PTY session pool           │
│ • Ring buffer (64 KB) per session for reconnection   │
│ • AI proxy → tck-ai → Ollama / Claude / OpenAI       │
│ • Serves the WASM bundle                             │
└──────────────────────┬───────────────────────────────┘
                       │
┌──────────────────────▼───────────────────────────────┐
│ tck-ai (TypeScript, Vercel AI SDK)                    │
│ • Unified provider access                             │
│ • models.dev model catalog                            │
└──────────────────────────────────────────────────────┘
```

| Crate           | Target                   | Responsibility                       |
|-----------------|--------------------------|--------------------------------------|
| `tck-core`      | both                     | Shared serde types, provider presets |
| `tck-ui`        | `wasm32-unknown-unknown` | Dioxus UI compiled to WebAssembly    |
| `tck-server`    | native                   | axum server: AI proxy + PTY + static |
| `tck-desktop`   | native + webview         | Desktop app (embeds server + WASM)   |

## Quick start

### Prerequisites

- [Rust](https://rustup.rs) (stable)
- WASM target: `rustup target add wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev): `cargo install trunk`

### Build & run

```bash
# 1. Build the WebAssembly UI
cd crates/tck-ui
trunk build --release
cd ../..

# 2. Run the server (serves the UI + provides terminals/AI)
cargo run -p tck-server --release

# 3. Open the app
#    http://127.0.0.1:3000
```

### Multi-instance dashboard

Once the server is running, open the app and you'll see the **Dashboard** view:

- Click **⚡ Claude Code**, **🛠 Aider**, **🧩 Codex**, etc. to launch that tool
  in its own terminal session.
- Switch between running instances using the sidebar.
- Click **💬 New AI Chat** for a standalone AI conversation.
- Use the Dispatch bar to send instructions to any instance.
- Switch to **Editor** or **Orchestrator** views using the top nav bar.

> Each terminal instance runs as a detached OS process — closing the browser
> tab leaves it running. Reconnect later to see its output via the ring buffer.

### Live-reload development

```bash
# Terminal 1 — the backend
cargo run -p tck-server

# Terminal 2 — the UI with hot reload (proxies /api and /ws to :3000)
cd crates/tck-ui
trunk serve
# open http://127.0.0.1:8080
```

## Desktop App

T.C.K can run as a native desktop application with no browser chrome.

### Quick launcher (portable)

```bash
# Build and run the server, then open your browser:
cargo build -p tck-server --release
./target/release/tck-server
# Open http://127.0.0.1:3000 manually
```

### Native window (WebView2 / WebKit)

The `tck-desktop` crate wraps the server in a native webview window:

```bash
# Build the WASM UI first
cd crates/tck-ui && trunk build --release && cd ../..

# Build and run the desktop app
cargo run -p tck-desktop
```

This produces a single binary that embeds both the server and the static UI
assets. No browser tabs, no URL bar — just an app window.

> **Windows note:** Building native binaries requires the MSVC linker from
> [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/).

### Windows Installer

A full installer (Inno Setup) is available:

```powershell
# 1. Build the WASM UI and server
.\scripts\build.ps1

# 2. Compile the installer (requires Inno Setup)
ISCC.exe installer\windows\setup.iss
# Creates: dist\TCK-Setup-3.0.0.exe
```

The installer adds a Start Menu shortcut that launches the server and opens
your browser. See [installer/README.md](installer/README.md) for details.

### Build script

```powershell
.\scripts\build.ps1           # release build
.\scripts\build.ps1 -Dev       # debug build
.\scripts\build.ps1 -Desktop   # include tck-desktop
```

## Configuration

| Variable                   | Default              | Meaning                                        |
|----------------------------|----------------------|------------------------------------------------|
| `TCK_ADDR`                 | `127.0.0.1:3000`     | Address the server binds to                    |
| `TCK_DIST`                 | `crates/tck-ui/dist` | Directory of the built WASM bundle             |
| `TCK_GITHUB_CLIENT_ID`     | —                    | GitHub OAuth App client id (enables Git)       |
| `TCK_GITHUB_CLIENT_SECRET` | —                    | GitHub OAuth App client secret                 |
| `TCK_GITHUB_CALLBACK`      | derived from Host    | Override the OAuth callback URL (non-localhost)|
| `TCK_AI_BASE`              | `http://127.0.0.1:3002` | Base URL of the tck-ai AI service           |

Providers, API keys, and custom commands are configured in-app via **⚙ Settings**
and stored in the browser.

## Git & Scaffolding (GitHub OAuth)

The **Source Control** and **Scaffold** panels connect to GitHub through an
**OAuth App**. The client secret and your access token live only in the Rust
server — the browser holds just an httpOnly session cookie.

**One-time setup:**

1. Create an OAuth App at **GitHub → Settings → Developer settings → OAuth Apps →
   New OAuth App**:
   - *Homepage URL:* `http://localhost:3000`
   - *Authorization callback URL:* `http://localhost:3000/auth/github/callback`
2. Copy the **Client ID** and generate a **Client secret**, then run the server:

   ```bash
   TCK_GITHUB_CLIENT_ID=<id> TCK_GITHUB_CLIENT_SECRET=<secret> cargo run -p tck-server --release
   ```

3. Open `http://127.0.0.1:3000`, click **🔗 Connect GitHub**, and authorize.

**What you can do once connected:**

- Browse your repositories and folders; click a file to open it in the editor.
- Edit it and click **⬆ Commit** to push the change back.
- **Scaffold** a template into the editor, or push a starter project to a new repo.

## Platforms

### macOS / Linux

Build and run exactly as above — `tck-server` is cross-platform. The terminal
uses your `$SHELL` (zsh on macOS by default, falling back to `/bin/bash`).

### iPhone / iPad (and any phone)

The UI is responsive and works in **iOS Safari** (and Android Chrome). Since a
phone can't run the native server itself, point it at a T.C.K server on your
**same Wi-Fi**:

```bash
# Bind to all interfaces so the phone can reach you
TCK_ADDR=0.0.0.0:3000 cargo run -p tck-server --release
```

Then on the phone open `http://<your-computer-LAN-IP>:3000` (e.g.
`http://192.168.1.20:3000`).

On a phone the layout collapses to a bottom tab bar — **Agents · Editor ·
Terminal · AI** — plus the new **Dashboard** view for multi-instance management.

> Use `http://` on the LAN (not `https://`) so the terminal's `ws://` WebSocket
> connects. The server has no authentication — only run it on a trusted network.

## Hosted on GitHub Pages

The UI is also published as a static site:
**<https://raghuramcoding.github.io/keyboard/>**

T.C.K detects at runtime whether a native `tck-server` is reachable:

- **With the local server:** AI is proxied through the server and terminals work.
- **On GitHub Pages** (static, no server): the editor, settings, AI chat, and
  multi-instance dashboard (browser-only mode) work, but AI calls go **directly
  from the browser**:
  - ✅ **Anthropic Claude** (browser-access header sent automatically)
  - ✅ **OpenRouter**, local **Ollama** with `OLLAMA_ORIGINS=*`
  - ❌ Providers that block browser CORS — use the local server for those
  - ❌ **Terminal instances are unavailable** on Pages (they need the native PTY)

> Deploying under a different repo name? Change `--public-url /<repo>/` in the
> Pages workflow to match.

## Deploy to your own server (Hetzner, VPS, …)

To run the **full** app (terminal + Git + AI) on the internet you need a real
server. A `Dockerfile`, `docker-compose.yml`, Caddy config (auto-HTTPS +
basic-auth), and a `systemd` unit are included:

```bash
git clone https://github.com/Raghuramcoding/keyboard.git && cd keyboard
cp deploy/.env.example .env          # set TCK_DOMAIN (+ optional GitHub OAuth)
docker compose up -d --build
```

Full instructions in **[deploy/README.md](deploy/README.md)**.

> ⚠️ The terminal is a real shell and the app has **no built-in auth** — anyone
> who can reach an open instance gets a shell. Always keep it behind basic-auth
> and/or a firewall.

## License

MIT — see [LICENSE](LICENSE).
