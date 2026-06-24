# Contributing to Keyboard

Thanks for your interest in contributing! This is a community project and
contributions of all kinds are welcome — bug reports, features, docs, and fixes.

## Getting started

You only need **Node.js 18+** to develop. Python is **not** required.

```bash
git clone https://github.com/Raghuramcoding/keyboard.git
cd keyboard
npm install
npm run dev      # build + launch with auto-rebuild
```

## Project layout

| Path | What it is |
|------|-----------|
| `src/main.ts` | Electron main process — native backend (AI providers, PTY, agents, settings, dialogs) |
| `src/preload.ts` | contextBridge API exposed to the renderer |
| `src/renderer/App.tsx` | Main UI (editor, chat, git, scaffold, toolbar) |
| `src/renderer/Terminal.tsx` | Embedded PTY terminal (xterm.js) |
| `src/renderer/Settings.tsx` | Settings window incl. custom providers |
| `build.js` | esbuild bundling (renderer + Monaco workers + main/preload) |

## Before you open a PR

Please make sure these pass:

```bash
npm run build        # esbuild bundles cleanly
npm run typecheck    # tsc --noEmit is clean
KEYBOARD_SMOKE=1 npx electron .    # headless smoke test prints SMOKE_RESULT PASS
```

The smoke test verifies the UI mounts, Monaco + xterm load, a real PTY runs, the
two-agent behavior works, and an OpenAI-compatible provider round-trips.

## Guidelines

- Keep changes focused; match the surrounding code style.
- The renderer is sandboxed (`contextIsolation: true`) — all OS/Node access goes
  through `preload.ts` → IPC handlers in `main.ts`.
- Prefer shell-agnostic commands (the default shell is PowerShell on Windows).
- Native modules must use ABI-stable N-API prebuilds so packaging needs no compiler.

## Reporting bugs / requesting features

Use the issue templates. Include your OS, Node version, and steps to reproduce.
