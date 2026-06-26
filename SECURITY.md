# Security Policy — T.C.K (TalentCloud Keyboard)

## Reporting a vulnerability

Please **do not** open a public issue for security vulnerabilities. Instead, use
GitHub's private vulnerability reporting (the **Security** tab → *Report a
vulnerability*), or contact the maintainer directly.

We aim to acknowledge reports within a few days.

## Notes on this app

- **The terminal runs a real shell with your user privileges.** The `tck-server`
  PTY bridge spawns PowerShell/bash and streams it to the browser — only run
  commands and agents you trust, and only connect to a server you control.
- **Bind locally.** The server defaults to `127.0.0.1:3000`. Do not expose it to a
  public network: anyone who can reach it gets shell access. There is currently no
  authentication on the `/ws/pty` or `/api/generate` endpoints.
- **API keys** (Anthropic, OpenAI-compatible providers) are stored in your
  browser's `localStorage` and sent to `tck-server` only to make the upstream
  request. They are never committed to the repo.
- The UI is a sandboxed WebAssembly module in the browser; all OS access (shell,
  outbound HTTP to providers) happens in the native `tck-server` process.
