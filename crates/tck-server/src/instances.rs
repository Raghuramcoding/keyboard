//! Multi-instance manager — each terminal tool gets its own PTY session.
//!
//! The [`InstanceManager`] holds a set of running PTY sessions keyed by
//! instance ID. It handles lifecycle: spawn, stdin write, kill, cleanup.
//!
//! Each PTY runs in its own OS process (detached from any one WebSocket),
//! so closing the browser tab leaves the tool running — reconnect later.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tck_core::{AgentState, CreateInstanceRequest, InstanceInfo, InstanceKind, InstanceStatus};
use tokio::sync::{broadcast, mpsc};

use regex::Regex;

// ---------------------------------------------------------------------------
// Constants

/// Maximum bytes retained per PTY for reconnection replay.
const RING_BUF_CAP: usize = 64 * 1024;

// Status byte constants (stored in AtomicU8).
const S_RUNNING: u8 = 0;
const S_STOPPED: u8 = 1;
const S_ERROR: u8 = 2;

// Agent state byte constants (stored in AtomicU8 on PtySession).
const AS_IDLE: u8 = 0;
const AS_WORKING: u8 = 1;
const AS_DONE: u8 = 2;
const AS_BLOCKED: u8 = 3;
const AS_ERROR: u8 = 4;

// ---------------------------------------------------------------------------
// AgentState <-> wire byte conversion

/// Map an [`AgentState`] to the wire byte used in [`AtomicU8`].
fn agent_state_to_u8(s: &AgentState) -> u8 {
    match s {
        AgentState::Idle => AS_IDLE,
        AgentState::Working => AS_WORKING,
        AgentState::Done => AS_DONE,
        AgentState::Blocked => AS_BLOCKED,
        AgentState::Error => AS_ERROR,
    }
}

fn u8_to_agent_state(n: u8) -> AgentState {
    match n {
        AS_IDLE => AgentState::Idle,
        AS_WORKING => AgentState::Working,
        AS_DONE => AgentState::Done,
        AS_BLOCKED => AgentState::Blocked,
        AS_ERROR => AgentState::Error,
        _ => AgentState::Idle,
    }
}

// ---------------------------------------------------------------------------
// Regex-based heuristic detector for AI coding agent state from PTY output.

/// Looks for tell-tale patterns in terminal output to infer whether the
/// running agent is working, done, blocked waiting for user input, or hit
/// an error.
pub struct AgentStateDetector {
    idle_re: Regex,
    working_re: Regex,
    done_re: Regex,
    blocked_re: Regex,
    error_re: Regex,
    last_output: Instant,
    silence_threshold: Duration,
}

impl AgentStateDetector {
    pub fn new() -> Self {
        Self {
            idle_re: Regex::new(r"(?m)[$#>]$").unwrap(),
            working_re: Regex::new(r"(?i)thinking|generating|analyzing|processing").unwrap(),
            done_re: Regex::new(r"(?i)completed|finished|done!|summary:").unwrap(),
            blocked_re: Regex::new(r"(?i)y/n|choose|select|press enter").unwrap(),
            error_re: Regex::new(r"Error:|Failed:|Traceback|exit code").unwrap(),
            last_output: Instant::now(),
            silence_threshold: Duration::from_secs(5),
        }
    }

    /// Feed a chunk of PTY output and check for a state transition.
    ///
    /// Returns `Some(new_state)` when a transition is detected, or `None`
    /// if the current state should remain unchanged. The caller is expected
    /// to update its own `AtomicU8` when a transition is returned.
    pub fn feed(&mut self, chunk: &[u8], current_state: AgentState) -> Option<AgentState> {
        self.last_output = Instant::now();
        let text = String::from_utf8_lossy(chunk);

        // Error — highest priority; always report.
        if current_state != AgentState::Error && self.error_re.is_match(&text) {
            return Some(AgentState::Error);
        }

        // Working — output that looks like agent deliberation.
        if current_state != AgentState::Working && self.working_re.is_match(&text) {
            return Some(AgentState::Working);
        }

        // Done — explicit completion markers.
        if current_state != AgentState::Done && self.done_re.is_match(&text) {
            return Some(AgentState::Done);
        }

        // Blocked — agent has asked for user input.
        if current_state != AgentState::Blocked && self.blocked_re.is_match(&text) {
            return Some(AgentState::Blocked);
        }

        // Idle — prompt characters at end of a line (shell or REPL prompt).
        if current_state != AgentState::Idle && self.idle_re.is_match(&text) {
            return Some(AgentState::Idle);
        }

        None
    }

    /// Check whether the session has been silent long enough to be Idle.
    ///
    /// Call this from `to_info()` to catch idle-from-silence even when the
    /// reader thread hasn't received a matching prompt pattern.
    pub fn check_silence(&self, current_state: AgentState) -> Option<AgentState> {
        if matches!(current_state, AgentState::Working | AgentState::Blocked)
            && self.last_output.elapsed() > self.silence_threshold
        {
            return Some(AgentState::Idle);
        }
        None
    }
}

// ---------------------------------------------------------------------------
// Handle to one running PTY session.

/// Internal PTY session state.
struct PtySession {
    id: String,
    label: String,
    cmd: String,
    created_at: u64,
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// The master end of the PTY, retained so we can send resize events
    /// to the kernel (and thus to the child process).
    master: Option<Box<dyn portable_pty::MasterPty + Send>>,
    buffer: Arc<Mutex<VecDeque<u8>>>,
    tx: broadcast::Sender<Vec<u8>>,
    status: Arc<AtomicU8>,
    agent_state_detector: Arc<Mutex<AgentStateDetector>>,
    current_agent_state: Arc<AtomicU8>,
    /// Unix timestamp (seconds) of the last byte read from the PTY.
    last_output_at: Arc<AtomicU64>,
    _reader_handle: Option<std::thread::JoinHandle<()>>,
}

impl PtySession {
    /// Spawn a new PTY running `command`. The reader thread runs in the
    /// background pushing output into both the ring buffer and a broadcast
    /// channel so that (re)connecting WebSockets can catch up.
    ///
    /// If `event_tx` is provided, agent-state transitions detected from PTY
    /// output are serialised as [`EventMessage`] JSON strings and sent there.
    fn spawn(
        id: String,
        label: String,
        command: String,
        cwd: Option<&str>,
        event_tx: Option<mpsc::UnboundedSender<String>>,
        bcast_tx: broadcast::Sender<String>,
    ) -> Result<Self, String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize { rows: 30, cols: 110, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| format!("openpty: {e}"))?;

        let mut cmd = if cfg!(windows) {
            // Wrap the command in PowerShell so PATH resolution works.
            let mut ps = CommandBuilder::new("pwsh");
            ps.arg("-NoProfile");
            ps.arg("-Command");
            ps.arg(&command);
            ps
        } else {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
            let mut c = CommandBuilder::new(&shell);
            c.arg("-c");
            c.arg(&command);
            c
        };
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        } else if let Ok(cwd) = std::env::current_dir() {
            cmd.cwd(cwd);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("spawn: {e}"))?;

        let mut reader = pair.master.try_clone_reader().expect("clone pty reader");
        let writer = Arc::new(Mutex::new(pair.master.take_writer().expect("take pty writer")));
        let master: Option<Box<dyn portable_pty::MasterPty + Send>> = Some(pair.master);

        let buffer: Arc<Mutex<VecDeque<u8>>> = Arc::new(Mutex::new(VecDeque::with_capacity(RING_BUF_CAP)));
        let (tx, _) = broadcast::channel(1024);
        let status: Arc<AtomicU8> = Arc::new(AtomicU8::new(S_RUNNING));

        // Liveness — track the last time we read output.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last_output_at: Arc<AtomicU64> = Arc::new(AtomicU64::new(now));

        // Agent state detection.
        let agent_state_detector: Arc<Mutex<AgentStateDetector>> =
            Arc::new(Mutex::new(AgentStateDetector::new()));
        let current_agent_state: Arc<AtomicU8> = Arc::new(AtomicU8::new(AS_IDLE));

        // Background reader thread: reads PTY → ring buffer + broadcast + state detection.
        let buf_w = buffer.clone();
        let tx_w = tx.clone();
        let st_w = status.clone();
        let detector_w = agent_state_detector.clone();
        let state_w = current_agent_state.clone();
        let last_output_w = last_output_at.clone();
        let event_tx_w = event_tx.clone();
        let bcast_tx_w = bcast_tx.clone();
        let instance_id = id.clone();
        let reader_handle = std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => {
                        st_w.store(S_STOPPED, Ordering::Relaxed);
                        break;
                    }
                    Ok(n) => {
                        let chunk = buf[..n].to_vec();

                        // Update last-output timestamp (process liveness).
                        let ts = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        last_output_w.store(ts, Ordering::Relaxed);

                        // Feed chunk to state detector and emit events on transitions.
                        let mut detector = detector_w.lock().unwrap();
                        let cur = u8_to_agent_state(state_w.load(Ordering::Relaxed));
                        if let Some(new_state) = detector.feed(&chunk, cur) {
                            state_w.store(agent_state_to_u8(&new_state), Ordering::Relaxed);
                            // Publish event to both mpsc (server event_rx) and broadcast (WS).
                            let msg = tck_core::EventMessage::AgentStateChanged {
                                instance_id: instance_id.clone(),
                                old_state: cur,
                                new_state,
                                hint: String::new(),
                                timestamp: ts,
                            };
                            if let Ok(json) = serde_json::to_string(&msg) {
                                if let Some(ref tx) = event_tx_w {
                                    let _ = tx.send(json.clone());
                                }
                                let _ = bcast_tx_w.send(json);
                            }
                        }
                        drop(detector);

                        // Ring buffer (last 64KB).
                        let mut ring = buf_w.lock().unwrap();
                        ring.extend(&chunk);
                        while ring.len() > RING_BUF_CAP {
                            let excess = ring.len() - RING_BUF_CAP;
                            ring.drain(0..excess);
                        }
                        drop(ring);
                        // Broadcast to any connected WS tasks.
                        let _ = tx_w.send(chunk);
                    }
                }
            }
        });

        Ok(PtySession {
            id,
            label,
            cmd: command,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            child: Some(child),
            writer,
            master,
            buffer,
            tx,
            status,
            agent_state_detector,
            current_agent_state,
            last_output_at,
            _reader_handle: Some(reader_handle),
        })
    }

    fn status(&self) -> InstanceStatus {
        match self.status.load(Ordering::Relaxed) {
            S_STOPPED => InstanceStatus::Stopped,
            S_ERROR => InstanceStatus::Error,
            _ => InstanceStatus::Running,
        }
    }

    fn to_info(&self) -> InstanceInfo {
        // Check for silence-induced idle transition.
        let detector = self.agent_state_detector.lock().unwrap();
        let raw_state = u8_to_agent_state(self.current_agent_state.load(Ordering::Relaxed));
        let agent_state = detector.check_silence(raw_state).unwrap_or(raw_state);
        drop(detector);

        InstanceInfo {
            id: self.id.clone(),
            kind: InstanceKind::Terminal,
            label: self.label.clone(),
            command: Some(self.cmd.clone()),
            cwd: None,
            status: self.status(),
            error: None,
            created_at: self.created_at,
            agent_state,
            last_output_at: self.last_output_at.load(Ordering::Relaxed),
        }
    }

    /// Resize the PTY terminal dimensions.
    ///
    /// Propagates the new size to the kernel so that the child process
    /// receives `SIGWINCH` (on Unix) or the equivalent resize notification
    /// (on Windows).
    pub fn resize(&self, rows: u16, cols: u16, pw: u16, ph: u16) -> Result<(), String> {
        if let Some(ref master) = self.master {
            master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: pw,
                    pixel_height: ph,
                })
                .map_err(|e| format!("resize: {e}"))
        } else {
            Err("PTY master handle is not available".into())
        }
    }
}

// ---------------------------------------------------------------------------
// Public API

/// Manages all running terminal instances.
pub struct InstanceManager {
    sessions: Arc<Mutex<HashMap<String, PtySession>>>,
    /// Optional channel for publishing instance-level events (agent state
    /// transitions, liveness changes, etc.) as JSON strings to the
    /// server's event receiver (consumed by the desktop app).
    event_tx: Option<mpsc::UnboundedSender<String>>,
    /// Broadcast channel for fanning out events to all WebSocket subscribers
    /// (e.g. `/ws/events`). Messages are the same JSON strings as `event_tx`.
    bcast_tx: broadcast::Sender<String>,
}

impl InstanceManager {
    /// Create a manager with no event publisher (backward-compatible default).
    pub fn new() -> Self {
        let (bcast_tx, _) = broadcast::channel(256);
        InstanceManager {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            event_tx: None,
            bcast_tx,
        }
    }

    /// Create a manager that publishes instance events on `event_tx`.
    pub fn with_event_tx(event_tx: Option<mpsc::UnboundedSender<String>>) -> Self {
        let (bcast_tx, _) = broadcast::channel(256);
        InstanceManager {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            bcast_tx,
        }
    }

    /// Create a new terminal instance. Returns its [`InstanceInfo`].
    pub fn create(&self, req: &CreateInstanceRequest) -> Result<InstanceInfo, String> {
        let id = format!("inst_{:016x}", rand_id());
        let label = req
            .label
            .clone()
            .unwrap_or_else(|| req.command.clone().unwrap_or_else(|| "terminal".into()));
        let command = req.command.clone().unwrap_or_else(|| {
            if cfg!(windows) { "pwsh".into() } else { std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into()) }
        });
        let session = PtySession::spawn(id.clone(), label, command, req.cwd.as_deref(), self.event_tx.clone(), self.bcast_tx.clone())?;
        let info = session.to_info();
        self.sessions.lock().unwrap().insert(id, session);
        Ok(info)
    }

    /// Return info for every running instance.
    pub fn list(&self) -> Vec<InstanceInfo> {
        self.sessions
            .lock()
            .unwrap()
            .values()
            .map(|s| s.to_info())
            .collect()
    }

    /// Return info for a single instance, or `None` if unknown.
    pub fn get(&self, id: &str) -> Option<InstanceInfo> {
        self.sessions.lock().unwrap().get(id).map(|s| s.to_info())
    }

    /// Kill and remove an instance.
    pub fn kill(&self, id: &str) -> Result<(), String> {
        let mut map = self.sessions.lock().unwrap();
        if let Some(session) = map.remove(id) {
            // Take the child so we can call kill() (which consumes the box).
            if let Some(mut child) = session.child {
                let _ = child.kill();
            }
            Ok(())
        } else {
            Err(format!("instance {id} not found"))
        }
    }

    /// Rename an instance (update its label).
    pub fn rename(&self, id: &str, label: &str) -> Result<(), String> {
        let mut map = self.sessions.lock().unwrap();
        if let Some(session) = map.get_mut(id) {
            session.label = label.to_string();
            Ok(())
        } else {
            Err(format!("instance {id} not found"))
        }
    }

    /// Resize an instance's PTY terminal dimensions.
    pub fn resize(
        &self,
        id: &str,
        rows: u16,
        cols: u16,
        pw: u16,
        ph: u16,
    ) -> Result<(), String> {
        let map = self.sessions.lock().unwrap();
        if let Some(session) = map.get(id) {
            session.resize(rows, cols, pw, ph)
        } else {
            Err(format!("instance {id} not found"))
        }
    }

    /// Write a line of text (plus newline) to the instance's stdin.
    pub fn write_stdin(&self, id: &str, input: &str) -> Result<(), String> {
        let map = self.sessions.lock().unwrap();
        if let Some(session) = map.get(id) {
            let mut w = session.writer.lock().unwrap();
            w.write_all(input.as_bytes())
                .and_then(|_| w.write_all(b"\n"))
                .and_then(|_| w.flush())
                .map_err(|e| format!("write stdin: {e}"))
        } else {
            Err(format!("instance {id} not found"))
        }
    }

    /// Subscribe to an instance's output stream.
    ///
    /// Returns `(initial_output, receiver, writer)`:
    /// - `initial_output` — ring-buffer replay for reconnection
    /// - `receiver` — live broadcast receiver for ongoing output
    /// - `writer` — shared writer handle for feeding stdin
    #[allow(clippy::type_complexity)]
    pub fn attach(
        &self,
        id: &str,
    ) -> Result<
        (
            Vec<u8>,
            broadcast::Receiver<Vec<u8>>,
            Arc<Mutex<Box<dyn Write + Send>>>,
        ),
        String,
    > {
        let map = self.sessions.lock().unwrap();
        if let Some(session) = map.get(id) {
            let replay: Vec<u8> = session.buffer.lock().unwrap().iter().copied().collect();
            let rx = session.tx.subscribe();
            let w = session.writer.clone();
            Ok((replay, rx, w))
        } else {
            Err(format!("instance {id} not found"))
        }
    }

    /// Subscribe to instance-level events (agent state transitions, etc.)
    /// through a broadcast channel. Multiple consumers can subscribe.
    ///
    /// The receiver yields JSON-encoded [`tck_core::EventMessage`] strings
    /// emitted on agent state transitions and other lifecycle events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<String> {
        self.bcast_tx.subscribe()
    }
}

// ---------------------------------------------------------------------------
// Helpers

fn rand_id() -> u128 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos()
        .hash(&mut hasher);
    let a = hasher.finish();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut hasher);
    let b = hasher.finish();
    ((a as u128) << 64) | b as u128
}