import { BaseComponent } from './base.js';
import { InstanceKind, InstanceStatus, type InstanceInfo } from '../../core/types.js';

// The output pane is plain text, so drop the control sequences a real shell
// emits: OSC strings (incl. VS Code's 633 shell integration) and CSI/escape codes.
const OSC = /\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)/g;
const CSI = /\x1b\[[0-9;?]*[ -/]*[@-~]/g;
const ESC = /\x1b[@-Z\\-_]/g;

function stripAnsi(data: string): string {
  return data.replace(OSC, '').replace(CSI, '').replace(ESC, '');
}

export class InstanceDashboard extends BaseComponent {
  private socket?: WebSocket;

  /** Close the terminal socket so a re-render does not leak connections. */
  dispose(): void {
    this.socket?.close();
    this.socket = undefined;
  }

  render(): void {
    this.element.innerHTML = `
      <div class="editor-tabs">
        <div class="tab-active">
          <span class="tab-lang-dot"></span>
          main.ts
          <span class="tab-underline"></span>
        </div>
        <span class="tab-breadcrumb">src/ui · main</span>
        <div class="spacer"></div>
        <button class="commit-btn">Commit</button>
      </div>
      <div class="editor-body">
        <div class="line-nums">
          ${Array.from({ length: 20 }, (_, i) => `<div>${i + 1}</div>`).join('')}
        </div>
        <div class="code-body">
          <div><span style="color:#6b7488">// Unified TypeScript Codebase</span></div>
          <div><span style="color:#6b7488">// T.C.K rewritten for efficiency.</span></div>
        </div>
      </div>
      <div class="terminal" id="terminal-container">
        <div class="terminal-header">
          <span class="terminal-dot"></span>
          <span class="terminal-title" id="terminal-status">Terminal — connecting…</span>
          <div class="spacer"></div>
          <span class="terminal-shell">shell</span>
        </div>
        <div class="terminal-output" id="terminal-output"></div>
        <div class="terminal-row">
          <span class="terminal-prompt">$</span>
          <input class="terminal-input" id="terminal-input" placeholder="type a command and press Enter…">
        </div>
      </div>
    `;

    void this.connectTerminal();
  }

  private async connectTerminal(): Promise<void> {
    const status = this.element.querySelector<HTMLElement>('#terminal-status');
    const output = this.element.querySelector<HTMLElement>('#terminal-output');
    const input = this.element.querySelector<HTMLInputElement>('#terminal-input');
    if (!status || !output || !input) return;

    let instance: InstanceInfo;
    try {
      const resp = await fetch('/api/instances', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ kind: InstanceKind.Terminal, label: 'Terminal' }),
      });
      if (!resp.ok) throw new Error(`server returned ${resp.status}`);
      instance = await resp.json() as InstanceInfo;
    } catch (err) {
      status.textContent = 'Terminal — unavailable (no local server)';
      output.textContent = `Could not start a terminal: ${err instanceof Error ? err.message : 'request failed'}\n`;
      input.disabled = true;
      return;
    }

    if (instance.status === InstanceStatus.Error) {
      status.textContent = 'Terminal — failed to start';
      output.textContent = `${instance.error ?? 'Unknown error'}\n`;
      input.disabled = true;
      return;
    }

    this.state.instances = [instance];
    this.state.persistInstances();

    const wsProto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const socket = new WebSocket(`${wsProto}//${window.location.host}/ws/pty/${instance.id}`);
    this.socket = socket;

    socket.addEventListener('open', () => {
      status.textContent = 'Terminal — connected';
      input.disabled = false;
    });
    socket.addEventListener('message', (event: MessageEvent<string>) => {
      output.textContent += stripAnsi(event.data);
      output.scrollTop = output.scrollHeight;
    });
    socket.addEventListener('close', () => {
      status.textContent = 'Terminal — disconnected';
      input.disabled = true;
    });

    input.addEventListener('keydown', (e: KeyboardEvent) => {
      if (e.key !== 'Enter') return;
      e.preventDefault();
      if (socket.readyState !== WebSocket.OPEN) return;
      socket.send(`${input.value}\n`);
      input.value = '';
    });
  }
}
