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
- 🐙 **Git via GitHub OAuth** — connect your GitHub account, browse your repos, open files into the editor, and commit changes back — all in the browser (the OAuth secret + token stay server-side in Rust).
- 🏗️ **Scaffolding** — generate starter projects (Rust CLI, static web, Node, Python) into the editor, or push a whole template to a brand-new GitHub repo.
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

| Variable                   | Default              | Meaning                                        |
|----------------------------|----------------------|------------------------------------------------|
| `TCK_ADDR`                 | `127.0.0.1:3000`     | Address the server binds to                    |
| `TCK_DIST`                 | `crates/tck-ui/dist` | Directory of the built WASM bundle             |
| `TCK_GITHUB_CLIENT_ID`     | —                    | GitHub OAuth App client id (enables Git)       |
| `TCK_GITHUB_CLIENT_SECRET` | —                    | GitHub OAuth App client secret                 |
| `TCK_GITHUB_CALLBACK`      | derived from Host    | Override the OAuth callback URL (non-localhost)|

Providers, API keys, and custom commands are configured in-app via **⚙ Settings**
and stored in the browser.

## Git & scaffolding (GitHub OAuth)

The **Source Control** and **Scaffold** panels (left sidebar; the **🧰 Tools** tab
on phones) connect to GitHub through an **OAuth App**. The client secret and your
access token live only in the Rust server — the browser holds just an httpOnly
session cookie.

**One-time setup:**

1. Create an OAuth App at **GitHub → Settings → Developer settings → OAuth Apps →
   New OAuth App**:
   - *Homepage URL:* `http://localhost:3000`
   - *Authorization callback URL:* `http://localhost:3000/auth/github/callback`
2. Copy the **Client ID** and generate a **Client secret**, then run the server with them:

   ```bash
   TCK_GITHUB_CLIENT_ID=<id> TCK_GITHUB_CLIENT_SECRET=<secret> cargo run -p tck-server --release
   ```

3. Open `http://127.0.0.1:3000`, click **🔗 Connect GitHub**, and authorize.

**What you can do once connected:**

- Browse your repositories and folders; click a file to open it in the editor.
- Edit it and click **⬆ Commit** to push the change back (with a commit message).
- **Scaffold** a template: *Load into editor*, or enter a name and *Create GitHub
  repo* to push a whole starter project to a new repository.

> Git requires the native server (it holds the OAuth secret), so it isn't
> available on the static GitHub Pages deployment. Scaffold → *Load into editor*
> works everywhere, since the templates are compiled into the WASM.
> For a non-localhost deploy, register that origin's callback URL and set
> `TCK_GITHUB_CALLBACK` (and use `https`).

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

## Hosted on GitHub Pages

The UI is also published as a static site:
**<https://raghuramcoding.github.io/keyboard/>**

Every push to `main` runs [`.github/workflows/pages.yml`](.github/workflows/pages.yml),
which builds the WASM bundle (`trunk build --release --public-url /keyboard/`) and
deploys it (Pages is auto-enabled on first run).

T.C.K detects at runtime whether a native `tck-server` is reachable:

- **With the local server** (`cargo run -p tck-server`, then open `http://127.0.0.1:3000`):
  AI is proxied through the server and the **terminal works**.
- **On GitHub Pages** (static, no server): the editor, settings, and AI chat work,
  but AI calls go **directly from the browser** to the provider, so the provider
  must allow browser CORS:
  - ✅ **Anthropic Claude** (T.C.K sends the `anthropic-dangerous-direct-browser-access` header)
  - ✅ **OpenRouter**, and a local **Ollama** started with `OLLAMA_ORIGINS=*`
  - ❌ Providers that block browser CORS (e.g. OpenAI's own API) — use the local server for those
  - The **terminal is unavailable** on Pages (it requires the native PTY server)

> Deploying under a different repo name? Change `--public-url /<repo>/` in the
> Pages workflow to match.

## Deploy to your own server (Hetzner, VPS, …)

To run the **full** app (terminal + Git + AI) on the internet you need a real
server, not serverless — the backend keeps a long-lived process, a terminal
WebSocket, and in-memory sessions. A `Dockerfile`, `docker-compose.yml`, Caddy
config (auto-HTTPS + basic-auth), and a `systemd` unit are included:

```bash
git clone https://github.com/Raghuramcoding/keyboard.git && cd keyboard
cp deploy/.env.example .env          # set TCK_DOMAIN (+ optional GitHub OAuth)
# paste a basic-auth hash into deploy/Caddyfile (see the guide)
docker compose up -d --build
```

Full instructions (Docker **and** no-Docker/systemd paths, OAuth callback,
hardening) are in **[deploy/README.md](deploy/README.md)**.

> ⚠️ The terminal is a real shell and the app has **no built-in auth** — anyone
> who can reach an open instance gets a shell. Always keep it behind basic-auth
> and/or a firewall, and run it as a non-root user (the provided configs do).
>
> It will **not** run on serverless hosts like Vercel (no persistent WebSocket/
> PTY, ephemeral state); only the static UI can go there, like the Pages build.

## License

MIT — see [LICENSE](LICENSE).
