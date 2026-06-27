# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**T.C.K (TalentCloud Keyboard)** is a 100% Rust AI-powered coding environment that runs in the browser. The UI compiles to WebAssembly (Dioxus); the backend is a native server (axum + tokio) that provides:

- Real shell terminal via pseudo-terminal (`portable-pty`) over WebSocket
- AI request proxying to multiple providers (Anthropic Claude, Ollama, OpenRouter, OpenAI-compatible endpoints)
- GitHub OAuth integration for repository browsing and scaffolding
- Settings persistence in browser localStorage
- Responsive 3-column desktop layout that collapses to touch-friendly tabs on mobile

## Architecture

```
Browser (WASM)              Server (Native Rust)
┌──────────────────┐       ┌─────────────────────────┐
│ tck-ui (Dioxus)  │◄─────►│ tck-server (axum)       │
│ • Editor         │  /api │ • AI proxy              │
│ • Terminal view  │  /ws  │ • PTY terminal bridge   │
│ • Settings       │       │ • Static file serving   │
└──────────────────┘       └─────────────────────────┘
```

### Crates

| Crate | Target | Purpose |
|-------|--------|---------|
| `tck-core` | Both (native + WASM) | Shared serde types, provider presets, agent list |
| `tck-ui` | `wasm32-unknown-unknown` | Dioxus UI compiled to WebAssembly |
| `tck-server` | Native | axum HTTP server, WebSocket PTY bridge, AI proxy |

## Prerequisites & Setup

1. **Rust (stable)** via [rustup](https://rustup.rs)
2. **WASM target**: `rustup target add wasm32-unknown-unknown`
3. **Trunk** (builds WASM): `cargo install trunk` or download prebuilt binary from [trunkrs.dev](https://trunkrs.dev)

## Common Commands

### Development (live reload)

```bash
# Terminal 1: Run backend server (watches port 3000)
cargo run -p tck-server
# Backend serves static files from crates/tck-ui/dist by default
# TCK_DIST=path overrides this

# Terminal 2: Run UI with hot reload (proxies API calls to :3000)
cd crates/tck-ui
trunk serve
# Open http://127.0.0.1:8080 in browser
```

### Building for production

```bash
# Build WASM UI (outputs to crates/tck-ui/dist)
cd crates/tck-ui
trunk build --release
cd ../..

# Build native server
cargo build -p tck-server --release
# Binary: target/release/tck-server
```

### Linting & checks

```bash
# Check native crates (tck-core, tck-server) for warnings
cargo clippy -p tck-core -p tck-server -- -D warnings

# Check entire workspace
cargo clippy -- -D warnings

# Format check
cargo fmt --check

# Format fix
cargo fmt
```

### Testing

The project currently has minimal test coverage. No standard test targets defined in Cargo.toml.

## Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `TCK_ADDR` | `127.0.0.1:3000` | Server bind address |
| `TCK_DIST` | `crates/tck-ui/dist` | Built WASM bundle location |
| `TCK_GITHUB_CLIENT_ID` | — | GitHub OAuth App ID (enables Git features) |
| `TCK_GITHUB_CLIENT_SECRET` | — | GitHub OAuth App secret |
| `TCK_GITHUB_CALLBACK` | Derived from Host header | OAuth callback URL override |

## Key File Locations

| Path | Purpose |
|------|---------|
| `crates/tck-core/src/lib.rs` | Shared types: `Agent`, `Provider`, presets |
| `crates/tck-ui/src/main.rs` | Dioxus UI root component |
| `crates/tck-ui/index.html` | HTML entry point (Trunk manages this) |
| `crates/tck-ui/assets/style.css` | Global styles |
| `crates/tck-server/src/main.rs` | axum server, AI proxy, PTY bridge |
| `deploy/` | Docker Compose, systemd, Caddy configs for production |
| `.github/workflows/` | CI/CD (build on push to main, Pages deploy, releases) |

## Architecture Notes

- **No JavaScript/TypeScript** — the entire codebase is Rust. UI logic lives in `tck-ui` (WASM target), OS/network access lives in `tck-server` (native).
- **Workspace strategy** — all three crates depend on shared types in `tck-core`. The UI crate has zero dependencies on the server; they communicate over HTTP/WebSocket.
- **WASM bundle** — Trunk compiles `tck-ui` to a static bundle that the `tck-server` serves from `crates/tck-ui/dist` (configurable via `TCK_DIST`).
- **Terminal over WebSocket** — the PTY session runs server-side; terminal I/O streams over a WebSocket to the browser (`portable-pty` crate).
- **AI proxy** — the server forwards requests to providers (Claude, Ollama, etc.) and returns responses to the UI. GitHub Pages deployment bypasses the server, so AI calls go directly from the browser (requires CORS-compatible providers).

## Known Platforms

- ✅ **Windows, macOS, Linux** — `tck-server` is cross-platform
- ✅ **iOS/Android** — responsive UI works in Safari/Chrome when pointed at a T.C.K server on the LAN
- ✅ **GitHub Pages** — static UI works (no terminal, limited to CORS-friendly AI providers)

## Deployment

See **[deploy/README.md](deploy/README.md)** for:
- Docker Compose (with Caddy + basic-auth)
- systemd service unit
- ⚠️ **Security note**: No built-in auth; terminal is a real shell. Always run behind authentication and keep it off the public internet unless hardened.

## Git & GitHub OAuth

Git features (repository browsing, commits, scaffolding to new repos) require OAuth:

1. Create an OAuth App: **GitHub Settings → Developer settings → OAuth Apps → New OAuth App**
2. Set Homepage URL: `http://localhost:3000`
3. Set Authorization callback: `http://localhost:3000/auth/github/callback`
4. Run server with credentials:
   ```bash
   TCK_GITHUB_CLIENT_ID=<id> TCK_GITHUB_CLIENT_SECRET=<secret> cargo run -p tck-server --release
   ```

OAuth token and client secret live in the Rust server only; browser holds an httpOnly session cookie.

## CI/CD

- **ci.yml** — on push/PR: runs clippy on native crates, builds WASM UI, builds native server (Ubuntu + Windows)
- **pages.yml** — on push to main: builds WASM UI with `--public-url /keyboard/` and deploys to GitHub Pages
- **release.yml** — creates releases with built artifacts

## Troubleshooting

**Trunk build fails on WASM:** Ensure `rustup target add wasm32-unknown-unknown` was run.

**Server won't start:** Check if port 3000 is already in use. Override with `TCK_ADDR=127.0.0.1:3001 cargo run -p tck-server`.

**UI can't connect to server:** Make sure the server is running on the same port and both HTTP (`/api`) and WebSocket (`/ws`) are accessible. On mobile over LAN, ensure firewall allows the connection.

**AI provider errors:** Check API key in Settings. On GitHub Pages (static), ensure the provider allows browser CORS (Claude does; OpenAI doesn't).
