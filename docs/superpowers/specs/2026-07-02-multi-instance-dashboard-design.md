# Multi-Instance Orchestrator Dashboard

**Date:** 2026-07-02
**Status:** Draft

## Overview

Rewrite the T.C.K app from a single-terminal AI coding environment into a
**multi-instance orchestrator dashboard** that lets the user launch, switch
between, and dispatch instructions to multiple coding tools simultaneously —
both terminal-based CLI tools (Claude Code, Aider, Codex, etc.) and AI chat
sessions — all from one browser tab.

## Architecture

```
┌───────────────────────────────────────────────────────────────────┐
│  Browser (WASM) — Instance Dashboard                               │
│  ┌──────────────┬────────────────────────────────────────────────┐ │
│  │ Instance     │  Instance Panel  (selected instance)           │ │
│  │ Sidebar      │                                                │ │
│  │              │  ┌─ Terminal / Chat view ────────────────────┐ │ │
│  │ ● Claude     │  │  (PTY output stream or chat history)      │ │ │
│  │   backend    │  └───────────────────────────────────────────┘ │ │
│  │ ● Aider      │  ┌─ Inline input ────────────────────────────┐ │ │
│  │   frontend   │  │  [type here...]                      ⏎    │ │ │
│  │ ● Chat-1     │  └───────────────────────────────────────────┘ │ │
│  │              │  ┌─ Dispatch bar ─────────────────────────────┐ │ │
│  │ [+ New]      │  │  [message]  ➤  [instances ▼]    [Send]   │ │ │
│  └──────────────┘  └────────────────────────────────────────────┘ │ │
└────────────────────┬──────────────────────────────────────────────┘ │
                     │ HTTP / WebSocket                               │
┌────────────────────▼──────────────────────────────────────────────┐ │
│  tck-server (Rust + axum)                                          │ │
│  ┌─────────────────────────────────────────────────────────────┐   │ │
│  │  Instance Manager                                             │   │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │   │ │
│  │  │ PTY Session │  │ PTY Session │  │ PTY Session │  ...      │   │ │
│  │  │ (Claude)    │  │ (Aider)     │  │ (Codex)     │          │   │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘          │   │ │
│  └─────────────────────────────────────────────────────────────┘   │ │
│  ┌─────────────────────────────────────────────────────────────┐   │ │
│  │  AI proxy (→ tck-ai / Vercel AI SDK)                        │   │ │
│  └─────────────────────────────────────────────────────────────┘   │ │
└─────────────────────────────────────────────────────────────────────┘ │
                                                                        │
┌───────────────────────────────────────────────────────────────────────┘
│
▼
tck-ai (TypeScript, Vercel AI SDK)  —  models.dev catalog
```

### Relationship to Existing Panels

The existing panels (Editor, Settings, GitHub, Orchestrator) are **not removed**.
They move into a collapsible top-level toolbar alongside the instance dashboard:

- A persistent top bar has: **Dashboard** (instances) | **Editor** | **Settings** | **Orchestrator**
- **Dashboard** is the new home view — the multi-instance orchestrator described here.
- The other tabs open the existing functionality (editor with scratch buffer, settings
  panel, orchestrator panel, GitHub source control).
- On mobile, these become entries in a bottom tab bar alongside the instance quick-switcher.
- The old single-terminal + chat panels are **replaced** by the instance dashboard
  (which supports multiple terminals and multiple chats).

### Key Architectural Decisions

1. **Multiple independent PTY sessions** — one per terminal-based instance.
   Each is a real OS process with its own stdin/stdout, isolated from others.
2. **Chat instances are browser-only state** — no server-side session for chats.
   They persist in `localStorage` and use the existing AI proxy for generation.
3. **Detached PTY model** — closing the browser tab doesn't kill the process
   (like tmux). The instance keeps running in the background.
4. **Instance ID as routing key** — all operations reference a unique
   `instance_id` that maps to either a server-side PTY or a browser-side chat.

## Instance Model

```rust
// In tck-core:

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstanceKind {
    Terminal,
    Chat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstanceStatus {
    Running,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    pub id: String,
    pub kind: InstanceKind,
    pub label: String,
    pub command: Option<String>,    // CLI tool launched (terminal only)
    pub cwd: Option<String>,        // Working directory (terminal only)
    pub status: InstanceStatus,
    pub error: Option<String>,      // Error message if status == Error
    pub created_at: u64,            // Unix ms timestamp
}
```

## Server-Side: Instance Manager

A new module `tck-server/src/instances.rs` manages PTY lifecycle:

### API Endpoints

| Method | Path | Request | Response |
|--------|------|---------|----------|
| `POST` | `/api/instances` | `{kind, command?, label?, cwd?}` | `InstanceInfo` with new `id` |
| `GET` | `/api/instances` | — | `Vec<InstanceInfo>` |
| `GET` | `/api/instances/{id}` | — | `InstanceInfo` |
| `PATCH` | `/api/instances/{id}` | `{label?, command?}` | `InstanceInfo` |
| `DELETE` | `/api/instances/{id}` | — | `{ok: true}` |
| `POST` | `/api/instances/{id}/stdin` | `{input: string}` | `{ok: true}` |
| `WS` | `/ws/pty/{id}` | — | Bidirectional terminal I/O stream |

### PTY Lifecycle

1. **Create** → `POST /api/instances` with `kind: "terminal"` and `command`
   - Server spawns a PTY using `portable-pty`
   - Runs the requested command via the user's shell
   - PTY starts in a configurable working directory (default: server's CWD)
   - Returns the `id` immediately

2. **Connect** → Client opens `WebSocket /ws/pty/{id}`
   - Server streams PTY stdout to the WebSocket
   - Client sends stdin keystrokes via WebSocket
   - Same pattern as the current single-terminal code, but keyed by instance ID

3. **Detach / Reconnect** → WebSocket closes
   - PTY **keeps running** (process is orphaned, stays alive)
   - Output is buffered (ring buffer, last 64KB retained for history)
   - Client reconnects → receives buffered missed output + live stream
   - Client can see instance status via REST API without connecting WS

4. **Kill** → `DELETE /api/instances/{id}`
   - Sends SIGTERM to the child process, waits 5s, then SIGKILL
   - Cleans up the PTY master fd
   - Removes from the session map

### Data Structures

```rust
struct InstanceManager {
    sessions: HashMap<String, PtySession>,
}

struct PtySession {
    cmd: String,
    label: String,
    child: Box<dyn ChildKiller>,
    reader: JoinHandle<()>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    buffer: Arc<Mutex<VecDeque<u8>>>,  // ring buffer, last 64KB
    status: Arc<AtomicU8>,             // RUNNING=0, STOPPED=1, ERROR=2
    ws_connections: Arc<AtomicUsize>,  // for detecting detach / reconnect
}
```

### Instance Manager API

```rust
impl InstanceManager {
    fn new() -> Self;
    fn create(&mut self, command: &str, label: &str, cwd: Option<&str>)
        -> Result<String, String>;   // returns instance ID
    fn list(&self) -> Vec<InstanceInfo>;
    fn get(&self, id: &str) -> Option<&PtySession>;
    fn kill(&mut self, id: &str) -> Result<(), String>;
    fn rename(&mut self, id: &str, label: &str) -> Result<(), String>;
    fn write_stdin(&self, id: &str, input: &str) -> Result<(), String>;
    fn on_ws_connect(&self, id: &str) -> Result<PtyConnection, String>;  // returns reader/writer handles
}
```

## Client-Side: Instance Dashboard

### State Model

```rust
// In tck-ui/src/main.rs:

#[derive(Clone, Clone)]
struct InstanceState {
    info: InstanceInfo,
    // For terminal instances:
    term_lines: Signal<String>,
    term_connected: Signal<bool>,
    term_ws: Option<Coroutine<String>>,  // WebSocket sender
    // For chat instances:
    messages: Signal<Vec<ChatMsg>>,
    provider: Signal<String>,
    model: Signal<String>,
}
```

### Key State Changes

The current `AppState` replaces the single terminal with a multi-instance model:

```rust
struct AppState {
    instances: Signal<Vec<InstanceState>>,     // all instances
    active_instance: Signal<Option<String>>,   // selected instance ID
    // Kept from current state:
    settings: Signal<Settings>,
    code: Signal<String>,
    filename: Signal<String>,
    chat: Signal<Vec<ChatMsg>>,  // still used for the original AI chat tab
    busy: Signal<bool>,
    show_settings: Signal<bool>,
    server: Signal<bool>,
    // ... GitHub, etc.
}
```

### UI Components

1. **`InstanceSidebar`** — left panel listing all instances
   - Grouped by status (Running, Stopped)
   - Each instance shows: icon (based on kind or tool), label, status dot
   - Click to select. Right-click menu: Rename, Kill/Delete, Duplicate
   - Icon and color derived from the tool/kind (reuses `agent_color`)
   - Active instance is highlighted

2. **`InstancePanel`** — main content, renders based on `active_instance`
   - **`TerminalInstancePanel`**: PTY output (monospace, scrollable) + input field
     - Same rendering as current terminal, but scoped to one instance
     - Shows "disconnected" badge when WebSocket is down
     - Shows connection status in the header
   - **`ChatInstancePanel`**: message history + input field
     - Same rendering as current AI chat, but scoped to one instance
     - Has provider/model selector in the header

3. **`DispatchBar`** — bottom input bar for sending to one or more instances
   - Textarea + instance selector (multi-select chips or dropdown)
   - "All" option to broadcast to every running instance
   - For terminals: sends text to stdin (appends `\n` automatically)
   - For chats: sends as a user message and triggers AI generation

4. **`NewInstanceDropdown`** — "+ New" button with menu
   - Items from `known_agents()` (Claude Code, Codex, etc.)
   - "Custom terminal command…" → shows a prompt to type any command + label
   - "New AI chat…" → creates a new chat instance with default provider

### Mobile Layout

On narrow screens the sidebar collapses to a bottom tab bar with top-level
entries: **Instances** | **Editor** | **Settings**. Within the Instances tab,
a horizontal scrollable pill-selector at the top of the screen lets the user
quick-switch between running instances, with a "+" pill to create new ones.
The instance input field and dispatch bar remain pinned at the bottom of the
viewport. This reuses the existing mobile tab pattern (`MobileTab` enum) with
an additional "Instances" tab that replaces the old "Agents" + "Terminal" + "AI"
tabs.

### Chat Instance Details

Chat instances are entirely browser-side:
- Created via `POST /api/instances` with `kind: "chat"` (server just records metadata)
- Messages stored in browser state (in-memory) + serialized to `localStorage`
- AI generation uses the existing `call_generate()` / `generate_direct()` pipeline
- Survives page refresh via `localStorage` persistence
- Provider and model are per-instance (different from the global settings)

### Terminal Instance Details

Terminal instances are server-backed:
- Created via `POST /api/instances` with `kind: "terminal"` + `command`
- WebSocket at `/ws/pty/{id}` connects after creation
- Input via either the WebSocket or `POST /api/instances/{id}/stdin`
- Reconnection: exponential backoff (1s, 2s, 4s, 8s, max 30s)
- On reconnect: missed output (from the ring buffer) is replayed, then live stream resumes

## Data Flow

### Creating a Terminal Instance

```
User: clicks "+ New" → "Claude Code"
  ↓
UI: sends POST /api/instances { kind: "terminal", command: "claude", label: "Claude Code" }
  ↓
Server: InstanceManager::create("claude", "Claude Code", None)
  → Spawns PTY
  → Launches "claude" (resolved via PATH)
  → Returns { id: "inst_abc123", kind: "terminal", status: "running", ... }
  ↓
UI: adds instance to instances[], auto-selects it
  → Opens WebSocket /ws/pty/inst_abc123
  → Streams output to term_lines
  ↓
User: sees "> Claude Code ready"  (or whatever the CLI tool prints)
```

### Dispatching to Multiple Instances

```
User: types "refactor the auth module" in dispatch bar
  → selects checkboxes: [x] Claude Code  [x] Aider  [ ] Chat-1
  → clicks "Send"
  ↓
UI: for each selected instance:
  Terminal instances → POST /api/instances/{id}/stdin { input: "refactor the auth module\n" }
  Chat instance → calls generate() with that prompt, adds to messages
  ↓
UI: shows toast "Sent to 2 instances"
  ↓
User: switches between tabs, watches each tool work on the task
```

### Error Handling

| Scenario | Behavior |
|----------|----------|
| PTY spawn fails | Status → "error", error field set with message. User can delete and retry. |
| Process exits (tool finishes) | Status → "stopped", exit code shown in terminal view |
| WebSocket disconnects | Instance shows "reconnecting…" badge. Auto-retry with backoff. PTY keeps running. |
| Server unreachable | All terminal instances show "disconnected" status. Chat instances still work (browser-only). |
| AI generation fails | Chat instance shows error in message history as an "error" role message. No crash. |
| localStorage full | Chat instance fallback to in-memory only; warn user once. |

## Implementation Plan

### Phase 1 — Server: Multi-PTY + Instance Manager
1. Add `InstanceInfo`, `InstanceKind`, `InstanceStatus` to `tck-core`
2. Create `tck-server/src/instances.rs` with `InstanceManager`
3. Refactor the current single-PTY code into the multi-PTY pattern
4. Add REST API endpoints to `main.rs`
5. Add per-instance WebSocket endpoint `/ws/pty/{id}`

### Phase 2 — UI: Instance Dashboard
1. Add `InstanceState` and multi-instance state to `AppState`
2. Build `InstanceSidebar` component
3. Build `InstancePanel` + `TerminalInstancePanel` + `ChatInstancePanel`
4. Build `DispatchBar`
5. Build `NewInstanceDropdown`
6. Wire up state management (create, select, switch, kill)

### Phase 3 — Chat Instances
1. Add chat instance creation (browser-side)
2. Wire `call_generate()` to chat instances
3. Add `localStorage` persistence for chat instances
4. Add per-instance provider/model selection

### Phase 4 — Polish
1. Auto-reconnection with backoff for terminal instances
2. Ring buffer for missed output on reconnect
3. Right-click context menus on instances (rename, kill)
4. Instance grouping and sorting
5. Mobile layout adaptation

## Open Questions / Future

- **Instance templates**: Save reusable instance configurations (e.g. "Claude Code in
  ~/projects/backend with env var X")
- **Instance sharing**: Share terminal/chat outputs with others via a link
- **Session recording**: Record and replay terminal sessions
- **Resource limits**: Cap PTY memory/CPU usage per instance
- **Docker support**: Each instance runs in its own container
