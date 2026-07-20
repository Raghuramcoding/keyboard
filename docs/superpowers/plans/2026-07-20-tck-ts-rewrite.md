# T.C.K TypeScript Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite the entire T.C.K codebase (Core, Server, UI) from Rust to TypeScript using a minimalist stack (Vanilla TS, Hono, node-pty).

**Architecture:** Unified TypeScript mono-repo. Hono for the backend (Server + AI service), Vanilla TS with direct DOM manipulation for the UI, and `node-pty` for terminal bridging.

**Tech Stack:** TypeScript, Hono, node-pty, Vite (UI bundling), @octokit/rest (GitHub API).

## Global Constraints
- **UI Framework:** None (Vanilla TS/HTML/CSS).
- **Server Framework:** Hono.
- **PTY Library:** `node-pty`.
- **Principles:** Ponytail (minimal code, reuse, stdlib first).
- **Outcome:** Full removal of Rust crates (`tck-core`, `tck-server`, `tck-ui`).

---

## File Map

### Core (Shared Types)
- `src/core/types.ts`: All shared interfaces (Agent, Role, GenerateRequest, etc.).

### Server (Hono)
- `src/server/main.ts`: Entry point, Hono app setup.
- `src/server/routes/ai.ts`: AI proxy and generation logic.
- `src/server/routes/instances.ts`: PTY instance management.
- `src/server/routes/github.ts`: GitHub OAuth and API integration.
- `src/server/services/pty.ts`: `node-pty` wrapper and WebSocket bridge.
- `src/server/services/persistence.ts`: Local state persistence.

### UI (Vanilla TS)
- `src/ui/main.ts`: App entry, routing, and global initialization.
- `src/ui/state.ts`: Global `AppState` singleton.
- `src/ui/terminal.ts`: Xterm.js integration and WebSocket client.
- `src/ui/components/base.ts`: Simple BaseComponent class.
- `src/ui/components/title-bar.ts`: Title bar component.
- `src/ui/components/sidebar.ts`: Sidebar component.
- `src/ui/components/instance-dashboard.ts`: Instance management view.
- `src/ui/components/right-panel.ts`: Chat/AI panel.
- `src/ui/components/settings-modal.ts`: Settings management.

---

## Implementation Tasks

### Task 1: Core Types Migration
**Files:**
- Create: `src/core/types.ts`
- Delete: `crates/tck-core/src/lib.rs`

**Interfaces:**
- Produces: `Agent`, `AgentRole`, `PlannedTask`, `Role`, `ChatMsg`, `CatalogModel`, `ProviderConfig`, `CustomCommand`, `Settings`, `GenerateRequest`, `GenerateResponse`, `InstanceKind`, `AgentState`, `InstanceStatus`, `InstanceInfo`, `EventMessage`.

- [ ] **Step 1: Define all shared types in `src/core/types.ts`**
```typescript
export interface Agent { id: string; label: string; command: string; icon: string; }
export interface AgentRole { id: string; name: string; icon: string; system_prompt: string; }
export interface PlannedTask { role: string; title: string; instructions: string; }
export enum Role { User, Ai, Err }
export interface ChatMsg { role: Role; text: string; }
export interface CatalogModel { id: string; provider: string; name: string; context_window: number; supports_tools: boolean; supports_streaming: boolean; }
export interface ProviderConfig { id: string; name: string; base_url: string; api_key: string; models: string[]; }
export interface CustomCommand { id: string; label: string; command: string; icon: string; }
export interface Settings { providers: ProviderConfig[]; commands: CustomCommand[]; claude_api_key: string; ollama_host: string; background: string; }
export interface GenerateRequest { provider: string; model: string; prompt: string; system?: string; base_url?: string; api_key?: string; temperature?: number; max_tokens?: number; }
export interface GenerateResponse { response?: string; error?: string; model?: string; }
export enum InstanceKind { Terminal, Chat }
export enum AgentState { Idle, Working, Done, Blocked, Error }
export enum InstanceStatus { Running, Stopped, Error }
export interface InstanceInfo { id: string; kind: InstanceKind; label: string; command?: string; cwd?: string; status: InstanceStatus; error?: string; created_at: number; agent_state: AgentState; last_output_at: number; }
export type EventMessage = 
  | { type: 'agent_state_changed'; instance_id: string; old_state: AgentState; new_state: AgentState; hint: string; timestamp: number; }
  | { type: 'instance_status_changed'; instance_id: string; status: InstanceStatus; timestamp: number; }
  | { type: 'output_activity'; instance_id: string; bytes_written: number; timestamp: number; }
  | { type: 'heartbeat'; timestamp: number; };
```
- [ ] **Step 2: Remove `crates/tck-core`**
```bash
rm -rf crates/tck-core
```
- [ ] **Step 3: Commit**
```bash
git add src/core/types.ts
git commit -m "core: migrate shared types to TS"
```

### Task 2: Server Foundation (Hono)
**Files:**
- Create: `src/server/main.ts`
- Modify: `tck-ai/src/server.ts` (integrate or link)

**Interfaces:**
- Consumes: `src/core/types.ts`
- Produces: Hono app instance.

- [ ] **Step 1: Setup Hono server in `src/server/main.ts`**
```typescript
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { serve } from '@hono/node-server';

const app = new Hono();
app.use('/*', cors());

app.get('/', (c) => c.text('T.C.K Server Running'));

serve({ fetch: app.fetch, port: 3000 }, () => {
  console.log('T.C.K Server listening on http://localhost:3000');
});
```
- [ ] **Step 2: Integrate `tck-ai` routes**
Move logic from `tck-ai/src/server.ts` into `src/server/routes/ai.ts` and mount it under `/api/ai`.
- [ ] **Step 3: Commit**
```bash
git add src/server/main.ts
git commit -m "server: setup Hono base server"
```

### Task 3: PTY & Terminal Bridge
**Files:**
- Create: `src/server/services/pty.ts`
- Create: `src/server/routes/instances.ts`

**Interfaces:**
- Consumes: `node-pty`, `ws`
- Produces: WebSocket handlers for terminal I/O.

- [ ] **Step 1: Implement `PtyService` in `src/server/services/pty.ts`**
Using `node-pty` to spawn shells and handle data streams.
- [ ] **Step 2: Implement `/ws/term` and `/api/instances` routes**
Handle creation and deletion of PTY instances.
- [ ] **Step 3: Commit**
```bash
git add src/server/services/pty.ts src/server/routes/instances.ts
git commit -m "server: implement PTY bridge and instance routes"
```

### Task 4: GitHub OAuth & API
**Files:**
- Create: `src/server/routes/github.ts`

**Interfaces:**
- Consumes: `@octokit/rest`
- Produces: OAuth flow and repo browsing endpoints.

- [ ] **Step 1: Implement GitHub OAuth flow**
- [ ] **Step 2: Implement repository browsing and committing endpoints**
- [ ] **Step 3: Commit**
```bash
git add src/server/routes/github.ts
git commit -m "server: implement GitHub integration"
```

### Task 5: UI Foundation & State
**Files:**
- Create: `src/ui/main.ts`
- Create: `src/ui/state.ts`
- Create: `index.html` (root)

**Interfaces:**
- Consumes: `src/core/types.ts`
- Produces: `AppState` singleton.

- [ ] **Step 1: Create `src/ui/state.ts`**
```typescript
import { Settings, InstanceInfo } from '../core/types';

export class AppState {
  instances: InstanceInfo[] = [];
  settings: Settings = { providers: [], commands: [], claude_api_key: '', ollama_host: '', background: '' };
  provider = 'claude';
  model = 'claude-opus-4-8';
  showSettings = false;

  persistInstances() {
    localStorage.setItem('tck.instances', JSON.stringify(this.instances));
  }
  loadInstances() {
    const data = localStorage.getItem('tck.instances');
    if (data) this.instances = JSON.parse(data);
  }
}
export const state = new AppState();
```
- [ ] **Step 2: Setup `src/ui/main.ts` and `index.html`**
- [ ] **Step 3: Commit**
```bash
git add src/ui/main.ts src/ui/state.ts index.html
git commit -m "ui: setup base state and entry point"
```

### Task 6: UI Components Rewrite
**Files:**
- Create: `src/ui/components/base.ts`
- Create: `src/ui/components/title-bar.ts`, `sidebar.ts`, `instance-dashboard.ts`, `right-panel.ts`, `settings-modal.ts`

**Interfaces:**
- Consumes: `AppState`
- Produces: Rendered DOM elements.

- [ ] **Step 1: Implement `BaseComponent`**
- [ ] **Step 2: Implement each component following the Rust Dioxus logic**
- [ ] **Step 3: Commit**
```bash
git add src/ui/components/*
git commit -m "ui: implement minimalist component system"
```

### Task 7: Terminal Integration
**Files:**
- Create: `src/ui/terminal.ts`

**Interfaces:**
- Consumes: `xterm.js`
- Produces: Interactive terminal view.

- [ ] **Step 1: Implement Xterm.js terminal client**
- [ ] **Step 2: Connect to `/ws/term`**
- [ la ] **Step 3: Commit**
```bash
git add src/ui/terminal.ts
git commit -m "ui: integrate xterm.js terminal"
```

### Task 8: Final Cleanup & Rust Removal
**Files:**
- Delete: `crates/tck-server`, `crates/tck-ui`, `crates/tck-desktop`, `Cargo.toml`

- [ ] **Step 1: Verify all functionality in TS**
- [ ] **Step 2: Delete all Rust-related files and directories**
- [ ] **Step 3: Final Commit**
```bash
git rm -rf crates/ Cargo.toml
git commit -m "cleanup: remove all Rust crates and cargo config"
```
