//! Multi-instance manager — each terminal tool gets its own PTY session.
//!
//! The [`InstanceManager`] holds a set of running PTY sessions keyed by
//! instance ID.  It handles lifecycle: spawn, stdin write, kill, cleanup.
//!
//! Each PTY runs in its own OS process (detached from any one WebSocket),
//! so closing the browser tab leaves the tool running — reconnect later.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tck_core::{CreateInstanceRequest, InstanceInfo, InstanceKind, InstanceStatus};
use tokio::sync::broadcast;


/// Maximum bytes retained per PTY for reconnection replay.
const RING_BUF_CAP: usize = 64 * 1024;

// Status byte constants (stored in AtomicU8).
const S_RUNNING: u8 = 0;
const S_STOPPED: u8 = 1;
const S_ERROR: u8 = 2;

// ---------------------------------------------------------------------------

/// Handle to one running PTY session.
struct PtySession {
    id: String,
    label: String,
    cmd: String,
    created_at: u64,
    child: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    buffer: Arc<Mutex<VecDeque<u8>>>,
    tx: broadcast::Sender<Vec<u8>>,
    status: Arc<AtomicU8>,
    _reader_handle: Option<std::thread::JoinHandle<()>>,
}

impl PtySession {
    /// Spawn a new PTY running `command`.  The reader thread runs in the
    /// background pushing output into both the ring buffer and a broadcast
    /// channel so that (re)connecting WebSockets can catch up.
    fn spawn(
        id: String,
        label: String,
        command: String,
        cwd: Option<&str>,
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

        let buffer: Arc<Mutex<VecDeque<u8>>> = Arc::new(Mutex::new(VecDeque::with_capacity(RING_BUF_CAP)));
        let (tx, _) = broadcast::channel(1024);
        let status: Arc<AtomicU8> = Arc::new(AtomicU8::new(S_RUNNING));

        // Background reader thread: reads PTY → ring buffer + broadcast.
        let buf_w = buffer.clone();
        let tx_w = tx.clone();
        let st_w = status.clone();
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
            buffer,
            tx,
            status,
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
        InstanceInfo {
            id: self.id.clone(),
            kind: InstanceKind::Terminal,
            label: self.label.clone(),
            command: Some(self.cmd.clone()),
            cwd: None,
            status: self.status(),
            error: None,
            created_at: self.created_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Public API

/// Manages all running terminal instances.
pub struct InstanceManager {
    sessions: Arc<Mutex<HashMap<String, PtySession>>>,
}

impl InstanceManager {
    pub fn new() -> Self {
        InstanceManager {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new terminal instance.  Returns its [`InstanceInfo`].
    pub fn create(&self, req: &CreateInstanceRequest) -> Result<InstanceInfo, String> {
        let id = format!("inst_{:016x}", rand_id());
        let label = req
            .label
            .clone()
            .unwrap_or_else(|| req.command.clone().unwrap_or_else(|| "terminal".into()));
        let command = req.command.clone().unwrap_or_else(|| {
            if cfg!(windows) { "pwsh".into() } else { std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into()) }
        });
        let session = PtySession::spawn(id.clone(), label, command, req.cwd.as_deref())?;
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
