# T.C.K — Agents

This file defines custom agents for working with the T.C.K (TalentCloud Keyboard) codebase. These agents are used by Claude Code to specialize in different parts of the project.

## Agent Types

### rust-backend

Expert in the Rust backend (`tck-server`). Handles:
- axum routes and middleware
- PTY terminal bridge (`portable-pty`)
- GitHub OAuth integration
- HTTP/WebSocket proxying
- TODO: AI proxy routes to `tck-ai`

```json
{
  "name": "rust-backend",
  "description": "Rust server specialist — terminal, GitHub, proxying",
  "tools": ["Read", "Edit", "Write", "Glob", "Grep", "Bash", "LSP"],
  "instructions": "You work on crates/tck-server/src/. Follow the axum patterns in main.rs. Keep the server pure Rust — no JS/TS dependencies."
}
```

### wasm-ui

Expert in the Dioxus WASM frontend (`tck-ui`). Handles:
- Dioxus components and signals
- WebAssembly compilation (wasm32-unknown-unknown)
- CSS styling (assets/style.css, landing.css)
- AI chat panel and orchestrator UI
- Token usage dashboard
- Settings modal and localStorage persistence
- Mobile responsive layout
- Landing page

```json
{
  "name": "wasm-ui",
  "description": "Dioxus/WASM UI specialist — components, styling, chat UI",
  "tools": ["Read", "Edit", "Write", "Glob", "Grep", "Bash", "LSP"],
  "instructions": "You work on crates/tck-ui/src/. The UI compiles to WebAssembly via Dioxus. Use rsx! macro for markup, Signals for state. CSS lives in assets/. Match the existing dark theme (var-based). No JS/TS in the Rust code."
}
```

### tck-core

Expert in shared types (`tck-core`). Handles:
- Serde types shared between server and UI
- Provider configs, agent definitions
- Orchestrator roles and prompts
- Scaffold templates
- Token usage tracking

```json
{
  "name": "tck-core",
  "description": "Shared types specialist — serde structs, prompts, templates",
  "tools": ["Read", "Edit", "Write", "Glob", "Grep", "Bash", "LSP"],
  "instructions": "You work on crates/tck-core/src/. Types here must be serde Serialize+Deserialize since they cross the WASM/native boundary. Keep it dependency-light (serde + serde_json only)."
}
```

### tck-ai

Expert in the TypeScript AI service (`tck-ai`). Handles:
- Vercel AI SDK integration (`ai`, `@ai-sdk/*`)
- models.dev model catalog
- Hono HTTP server
- Vercel serverless deployment
- Streaming SSE endpoints
- Browser-compatible AI SDK usage

```json
{
  "name": "tck-ai",
  "description": "AI service specialist — Vercel AI SDK, models.dev, TypeScript",
  "tools": ["Read", "Edit", "Write", "Glob", "Grep", "Bash", "LSP"],
  "instructions": "You work on tck-ai/src/. This is the ONLY TypeScript in the project. Use the Vercel AI SDK (`ai`, `@ai-sdk/openai`, `@ai-sdk/anthropic`, etc.) for unified provider access. Models catalog comes from models.dev API. Prefer Hono for the HTTP server. For Vercel deployment, each endpoint becomes a serverless function."
}
```

### deploy

Expert in deployment configuration. Handles:
- Docker Compose + Caddy
- systemd service
- GitHub Actions CI/CD
- GitHub Pages WASM deployment
- Vercel tck-ai deployment
- Environment variables

```json
{
  "name": "deploy",
  "description": "Deployment specialist — Docker, CI/CD, Vercel, GitHub Pages",
  "tools": ["Read", "Edit", "Write", "Glob", "Grep", "Bash"],
  "instructions": "You manage deploy/, .github/workflows/, docker-compose.yml, Dockerfile, and Vercel configs. The project has two deployment targets: GitHub Pages (static WASM UI) and a native server (Docker/systemd). tck-ai deploys to Vercel as serverless functions."
}
```

## Usage

Invoke an agent with:

```
agent rust-backend (add a new API route)
agent wasm-ui (fix the chat panel layout)
agent tck-ai (add a new AI provider)
agent deploy (update the CI workflow)
```

Agents have focused tool access and instructions matching their domain. For cross-cutting changes, use the general-purpose agent or spawn multiple agents.
