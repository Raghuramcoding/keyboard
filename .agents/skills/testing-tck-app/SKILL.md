---
name: testing-tck-app
description: How to build, run and end-to-end test the T.C.K TypeScript SPA + Hono server (landing page, PTY terminal, AI chat proxy, settings modal, theme toggle). Use when testing or manually verifying anything in this repo's `src/ui` or `src/server` code.
---

# Testing T.C.K locally

Note: `CLAUDE.md` / `AGENTS.md` still describe the older Rust/Dioxus (`cargo`, `trunk`) stack. The
runnable app is now the root TypeScript project (`index.html` → `src/ui/main.ts`, server
`src/server/main.ts`). Ignore the Rust instructions unless working under `crates/`.

## Run it

```bash
npm install                    # node 20 works; engine warnings are safe to ignore
npm run build && npm start     # Hono server on http://localhost:3000 serving dist/
# or: npm run dev:server  (tsx watch, :3000)  +  npm run dev  (Vite :8080, proxies /api and /ws)
```

Gotchas:
- Port 3000 is often already occupied by a previous run: `ss -ltnp | grep 3000` then kill the PID.
- Prefer the built `npm start` path for testing — it exercises the same static bundle users get. Remember to re-run `npm run build` after UI edits, otherwise you are testing a stale `dist/`.
- Useful smoke checks before opening a browser:
  `curl -s -o /dev/null -w '%{http_code}' localhost:3000/`,
  `curl -s -XPOST localhost:3000/api/instances -H 'Content-Type: application/json' -d '{"kind":"terminal","label":"Terminal"}'`,
  `curl -s -XPOST localhost:3000/api/ai/generate -H 'Content-Type: application/json' -d '{"provider":"claude","model":"claude-sonnet-4-5","prompt":"hi"}'`.

## UI paths (no login/auth of any kind)

- `/` shows the landing page (SPA routes on `location.hash`); the "Launch T.C.K" CTA and the nav "Launch App →" link both go to `#app`.
- `#app` renders titlebar + sidebar + editor + terminal + AI chat.
- Theme: the small ☀/☾ icon button immediately left of "Settings" in the titlebar toggles `documentElement.dataset.theme`. Editor and terminal panes are hardcoded dark hexes in `index.html`, so they intentionally stay dark in light mode — do not report that as a bug.
- Settings: "Settings" button in the titlebar. Fields: "Claude API key", "Ollama host". Save persists to `localStorage['tck.settings']`; Cancel / X / backdrop-click close without saving. Theme choice is NOT persisted.
- Terminal: header should read "Terminal — connected"; type into the `$` input and press Enter. Output is appended verbatim, so raw ANSI/OSC-633 escape sequences show up as literal text — a known cosmetic issue, and it may reappear after refactors.
- Chat: textarea + Send, Enter sends (Shift+Enter newlines). Bubbles are built with `textContent`, so HTML/`<script>` payloads must appear literally — good regression check for XSS.

## Devin Secrets Needed

- None for structural/UI testing. A happy-path AI completion requires a provider key
  (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GROQ_API_KEY`, …) exported to the server process, or a
  Claude key typed into Settings. Without keys, `/api/ai/generate` returns a JSON `error` (e.g.
  `x-api-key header is required`) which the chat panel renders as an assistant bubble — that is the
  expected no-key behaviour, not a crash.

## Suggested environment blueprint (none exists yet)

```yaml
maintenance: |
  npm install
knowledge:
  - name: build
    contents: |
      npm run build   # vite build → dist/
  - name: run
    contents: |
      npm start       # tsx src/server/main.ts, serves dist/ on :3000
  - name: typecheck
    contents: |
      npm run typecheck
```
