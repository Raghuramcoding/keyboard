# Security Policy

## Reporting a vulnerability

Please **do not** open a public issue for security vulnerabilities. Instead, use
GitHub's private vulnerability reporting (the **Security** tab → *Report a
vulnerability*), or contact the maintainer directly.

We aim to acknowledge reports within a few days.

## Notes on this app

- API keys (Anthropic, OpenAI-compatible providers) are stored locally in your
  user-data `settings.json` and never committed to the repo.
- The embedded terminal runs a real shell with your user privileges — only run
  commands and agents you trust.
- The renderer is sandboxed (`contextIsolation: true`, `nodeIntegration: false`);
  all privileged operations go through the preload bridge and main-process IPC.
