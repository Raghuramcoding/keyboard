# Contributing to T.C.K (TalentCloud Keyboard)

Thanks for your interest in contributing! This is a community project and
contributions of all kinds are welcome — bug reports, features, docs, and fixes.

## Getting started

You need **Rust (stable)** with the WebAssembly target and **Trunk**.

```bash
git clone https://github.com/Raghuramcoding/keyboard.git
cd keyboard
rustup target add wasm32-unknown-unknown
cargo install trunk            # or download a prebuilt Trunk binary

# Backend (terminal + AI proxy + serves the UI)
cargo run -p tck-server

# UI with hot reload (in another terminal)
cd crates/tck-ui && trunk serve   # http://127.0.0.1:8080
```

## Project layout

| Path | What it is |
|------|-----------|
| `crates/tck-core/src/lib.rs`   | Shared serde types, agent list, provider presets |
| `crates/tck-ui/src/main.rs`    | Dioxus UI compiled to WebAssembly (editor, chat, terminal, settings) |
| `crates/tck-ui/index.html`     | Trunk entry / HTML shell |
| `crates/tck-ui/assets/style.css` | Styling |
| `crates/tck-server/src/main.rs`| axum server — AI proxy, PTY-over-WebSocket, static serving |

## Before you open a PR

Please make sure these pass:

```bash
# Native crates: lint clean
cargo clippy -p tck-core -p tck-server -- -D warnings

# WASM UI builds
cd crates/tck-ui && trunk build --release && cd ../..

# Native server builds
cargo build -p tck-server --release
```

Then run the server and confirm the app loads at <http://127.0.0.1:3000>: the UI
mounts, the terminal connects (green dot), and an AI provider round-trips.

## Guidelines

- Keep changes focused; match the surrounding code style.
- The whole app is Rust — UI logic lives in `tck-ui` (WASM), OS access (shell,
  outbound HTTP) lives in `tck-server` (native). Don't add JavaScript/TypeScript.
- Prefer shell-agnostic commands (the default shell is PowerShell on Windows, bash elsewhere).

## Reporting bugs / requesting features

Use the issue templates. Include your OS, Rust version, and steps to reproduce.
