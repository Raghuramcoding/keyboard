import * as pty from 'node-pty';
import { WebSocket } from 'ws';
import { InstanceInfo, InstanceStatus, AgentState } from '../../core/types.js';

export class PtyService {
  private instances = new Map<string, { pty: any; ws: WebSocket }>();

  createInstance(id: string, command: string, cwd: string): any {
    const ptyProcess = pty.spawn(command || 'bash', [], {
      name: 'xterm-color',
      cols: 80,
      rows: 24,
      cwd: cwd || process.cwd(),
      env: process.env,
    });
    return ptyProcess;
  }

  handleTerminalWs(ws: WebSocket, instanceId: string) {
    const ptyProcess = this.instances.get(instanceId)?.pty;
    if (!ptyProcess) return;

    ptyProcess.onData((data: string) => {
      ws.send(data);
    });

    ws.on('message', (data) => {
      ptyProcess.write(data.toString());
    });

    ws.on('close', () => {
      ptyProcess.kill();
      this.instances.delete(instanceId);
    });
  }

  registerInstance(id: string, pty: any, ws: WebSocket) {
    this.instances.set(id, { pty, ws });
  }

  removeInstance(id: string) {
    const inst = this.instances.get(id);
    if (inst) {
      inst.pty.kill();
      this.instances.delete(id);
    }
  }
}

export const ptyService = new PtyService();
