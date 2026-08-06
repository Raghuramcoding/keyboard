import * as pty from 'node-pty';
import type { WebSocket } from 'ws';

type PtyProcess = pty.IPty;

interface PtySession {
  pty: PtyProcess;
  ws?: WebSocket;
  /** Recent output, replayed when a client (re)connects. */
  buffer: string;
}

const BUFFER_LIMIT = 64 * 1024;

export class PtyService {
  private sessions = new Map<string, PtySession>();

  createInstance(id: string, command: string, cwd: string): PtyProcess {
    const shell = command || process.env.SHELL || 'bash';
    const ptyProcess = pty.spawn(shell, [], {
      name: 'xterm-color',
      cols: 80,
      rows: 24,
      cwd: cwd || process.cwd(),
      env: process.env as Record<string, string>,
    });

    const session: PtySession = { pty: ptyProcess, buffer: '' };
    this.sessions.set(id, session);

    ptyProcess.onData((data: string) => {
      session.buffer = (session.buffer + data).slice(-BUFFER_LIMIT);
      if (session.ws?.readyState === 1) session.ws.send(data);
    });
    ptyProcess.onExit(() => {
      this.sessions.delete(id);
    });

    return ptyProcess;
  }

  handleTerminalWs(ws: WebSocket, instanceId: string) {
    const session = this.sessions.get(instanceId);
    if (!session) {
      ws.close(1008, 'unknown instance');
      return;
    }

    session.ws = ws;
    if (session.buffer) ws.send(session.buffer);

    ws.on('message', (data) => {
      session.pty.write(data.toString());
    });

    // Detach the socket but keep the shell alive so it survives a reload.
    ws.on('close', () => {
      if (session.ws === ws) session.ws = undefined;
    });
  }

  removeInstance(id: string) {
    const session = this.sessions.get(id);
    if (!session) return;
    session.ws?.close();
    session.pty.kill();
    this.sessions.delete(id);
  }
}

export const ptyService = new PtyService();
