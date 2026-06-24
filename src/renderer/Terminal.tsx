import React, { useRef, useEffect, useState, useImperativeHandle, forwardRef, useCallback } from 'react';
import { Terminal as XTerm } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import 'xterm/css/xterm.css';

export interface TerminalHandle {
  /**
   * Launch a command in the terminal. Reuses the active tab only if it's a fresh
   * shell that hasn't launched anything yet; otherwise opens a NEW tab and runs
   * there. This is what lets "click Claude, then click Codex" actually start both
   * — Codex gets its own tab instead of being typed into Claude's running session.
   */
  launchCommand: (cmd: string) => void;
  /** Open a fresh, empty terminal tab. */
  newSession: () => void;
}

interface TerminalPaneProps {
  cwd?: string;
  fontSize: number;
  accent: string;
}

interface Inst {
  term: XTerm;
  fit: FitAddon;
  ready: boolean;
  mode: string;
}

interface Session { id: string; title: string; }

const TERMINAL_THEME = {
  background: '#1a1a1a',
  foreground: '#d4d4d4',
  cursor: '#d4d4d4',
  selectionBackground: '#264f78',
  black: '#1e1e1e', red: '#f44747', green: '#6a9955', yellow: '#d7ba7d',
  blue: '#569cd6', magenta: '#c586c0', cyan: '#4ec9b0', white: '#d4d4d4',
  brightBlack: '#808080', brightRed: '#f44747', brightGreen: '#b5cea8',
  brightYellow: '#dcdcaa', brightBlue: '#9cdcfe', brightMagenta: '#c586c0',
  brightCyan: '#4ec9b0', brightWhite: '#ffffff',
};

let seq = 0;
const genId = () => `term-${Date.now()}-${seq++}`;

const TerminalPane = forwardRef<TerminalHandle, TerminalPaneProps>(({ cwd, fontSize, accent }, ref) => {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [activeId, setActiveId] = useState<string>('');
  const instances = useRef<Map<string, Inst>>(new Map());
  const containers = useRef<Map<string, HTMLDivElement>>(new Map());
  const activeIdRef = useRef('');
  // Per-session queued command (run once that session's PTY is ready).
  const pendingCmds = useRef<Map<string, string>>(new Map());
  // Sessions that have already had a command launched into them.
  const usedIds = useRef<Set<string>>(new Set());
  const cwdRef = useRef(cwd);
  cwdRef.current = cwd;
  activeIdRef.current = activeId;

  const flushPending = useCallback((id: string) => {
    const cmd = pendingCmds.current.get(id);
    if (cmd != null) {
      window.electronAPI.ptyWrite(id, cmd + '\r');
      pendingCmds.current.delete(id);
    }
  }, []);

  // Initialise an xterm + PTY for a session once its container mounts.
  const mount = useCallback((id: string, el: HTMLDivElement | null) => {
    if (!el) { containers.current.delete(id); return; }
    if (instances.current.has(id)) return;
    containers.current.set(id, el);

    const term = new XTerm({
      fontSize,
      fontFamily: "'Cascadia Code', 'Cascadia Mono', 'Consolas', monospace",
      theme: TERMINAL_THEME,
      cursorBlink: true,
      scrollback: 5000,
      allowProposedApi: true,
    });
    const fit = new FitAddon();
    term.loadAddon(fit);
    term.open(el);
    try { fit.fit(); } catch { /* container may be 0px briefly */ }

    const inst: Inst = { term, fit, ready: false, mode: 'pty' };
    instances.current.set(id, inst);

    term.onData((data) => window.electronAPI.ptyWrite(id, data));

    window.electronAPI
      .ptyCreate({ id, cwd: cwdRef.current, cols: term.cols, rows: term.rows })
      .then((res: any) => {
        inst.ready = true;
        inst.mode = res?.mode || 'pty';
        if (res?.mode === 'piped') {
          term.writeln('\x1b[33m[Running in piped-shell fallback mode — full-screen TUIs may not render]\x1b[0m');
        }
        flushPending(id); // run any command queued for THIS session, active or not
        if (activeIdRef.current === id) term.focus();
      })
      .catch((e: any) => term.writeln(`\x1b[31mFailed to start shell: ${e.message}\x1b[0m`));
  }, [fontSize, flushPending]);

  // Single global subscription for PTY output/exit; routes by id.
  useEffect(() => {
    const offData = window.electronAPI.onPtyData(({ id, data }) => {
      instances.current.get(id)?.term.write(data);
    });
    const offExit = window.electronAPI.onPtyExit(({ id }) => {
      const inst = instances.current.get(id);
      if (inst) inst.term.writeln('\r\n\x1b[90m[process exited]\x1b[0m');
    });
    return () => { offData?.(); offExit?.(); };
  }, []);

  // Resize active terminal when the pane resizes.
  useEffect(() => {
    const onResize = () => {
      const inst = instances.current.get(activeIdRef.current);
      if (!inst) return;
      try {
        inst.fit.fit();
        window.electronAPI.ptyResize(activeIdRef.current, inst.term.cols, inst.term.rows);
      } catch { /* ignore */ }
    };
    window.addEventListener('resize', onResize);
    const ro = new ResizeObserver(onResize);
    const el = containers.current.get(activeId);
    if (el?.parentElement) ro.observe(el.parentElement);
    onResize();
    return () => { window.removeEventListener('resize', onResize); ro.disconnect(); };
  }, [activeId]);

  // Fit + focus when switching tabs.
  useEffect(() => {
    const inst = instances.current.get(activeId);
    if (inst) {
      setTimeout(() => {
        try { inst.fit.fit(); window.electronAPI.ptyResize(activeId, inst.term.cols, inst.term.rows); } catch { /* ignore */ }
        inst.term.focus();
      }, 0);
    }
  }, [activeId]);

  // Apply font-size changes live.
  useEffect(() => {
    instances.current.forEach((inst, id) => {
      inst.term.options.fontSize = fontSize;
      try { inst.fit.fit(); window.electronAPI.ptyResize(id, inst.term.cols, inst.term.rows); } catch { /* ignore */ }
    });
  }, [fontSize]);

  const createSession = useCallback((cmd?: string) => {
    const id = genId();
    if (cmd != null) pendingCmds.current.set(id, cmd);
    setSessions((s) => [...s, { id, title: `Terminal ${s.length + 1}` }]);
    setActiveId(id);
    return id;
  }, []);

  const closeSession = useCallback((id: string) => {
    window.electronAPI.ptyKill(id);
    const inst = instances.current.get(id);
    if (inst) { try { inst.term.dispose(); } catch { /* ignore */ } }
    instances.current.delete(id);
    containers.current.delete(id);
    usedIds.current.delete(id);
    pendingCmds.current.delete(id);
    setSessions((prev) => {
      const next = prev.filter((s) => s.id !== id);
      if (activeIdRef.current === id) setActiveId(next.length ? next[next.length - 1].id : '');
      return next;
    });
  }, []);

  useImperativeHandle(ref, () => ({
    launchCommand: (cmd: string) => {
      const active = activeIdRef.current;
      // Reuse the active tab only if it's a fresh shell (nothing launched yet).
      if (active && !usedIds.current.has(active)) {
        usedIds.current.add(active);
        const inst = instances.current.get(active);
        if (inst && inst.ready) { window.electronAPI.ptyWrite(active, cmd + '\r'); inst.term.focus(); }
        else pendingCmds.current.set(active, cmd); // PTY still mounting — flush on ready
      } else {
        // Active tab is busy (or none) — open a new tab and run there.
        const id = createSession(cmd);
        usedIds.current.add(id);
      }
    },
    newSession: () => createSession(),
  }), [createSession]);

  // Start with one terminal.
  useEffect(() => { if (sessions.length === 0) createSession(); }, []); // eslint-disable-line

  return (
    <div style={styles.wrap}>
      <div style={styles.tabBar}>
        {sessions.map((s) => (
          <div
            key={s.id}
            onClick={() => setActiveId(s.id)}
            style={{ ...styles.tab, ...(s.id === activeId ? { ...styles.tabActive, borderTopColor: accent } : {}) }}
          >
            <span>🖥️ {s.title}</span>
            <span onClick={(e) => { e.stopPropagation(); closeSession(s.id); }} style={styles.tabClose} title="Close">×</span>
          </div>
        ))}
        <button onClick={() => createSession()} style={styles.newTab} title="New terminal">＋</button>
      </div>
      <div style={styles.body}>
        {sessions.map((s) => (
          <div
            key={s.id}
            ref={(el) => mount(s.id, el)}
            style={{ ...styles.pane, display: s.id === activeId ? 'block' : 'none' }}
          />
        ))}
      </div>
    </div>
  );
});

const styles: Record<string, React.CSSProperties> = {
  wrap: { display: 'flex', flexDirection: 'column', height: '100%', backgroundColor: '#1a1a1a' },
  tabBar: { display: 'flex', alignItems: 'stretch', backgroundColor: 'var(--bg-elevated)', borderBottom: '1px solid #3c3c3c', minHeight: 30, overflowX: 'auto' },
  tab: { display: 'flex', alignItems: 'center', gap: 6, padding: '4px 10px', fontSize: 12, color: '#aaa', cursor: 'pointer', borderTop: '2px solid transparent', borderRight: '1px solid #2a2a2a', whiteSpace: 'nowrap', userSelect: 'none' },
  tabActive: { backgroundColor: '#1a1a1a', color: '#fff' },
  tabClose: { fontSize: 14, color: '#888', padding: '0 2px', borderRadius: 3 },
  newTab: { background: 'none', border: 'none', color: '#aaa', fontSize: 16, cursor: 'pointer', padding: '0 12px' },
  body: { flex: 1, position: 'relative', overflow: 'hidden', padding: '4px 6px' },
  pane: { position: 'absolute', inset: 0, padding: '2px 4px' },
};

export default TerminalPane;
