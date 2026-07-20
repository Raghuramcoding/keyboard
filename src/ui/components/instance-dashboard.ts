import { BaseComponent } from './base.js';
import { InstanceInfo, AgentState } from '../../core/types.js';

export class InstanceDashboard extends BaseComponent {
  render(): void {
    this.element.innerHTML = `
      <div class="main">
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
            ${Array.from({length: 20}, (_, i) => `<div>${i+1}</div>`).join('')}
          </div>
          <div class="code-body">
            <div><span style="color:#6b7488">// Unified TypeScript Codebase</span></div>
            <div><span style="color:#6b7488">// T.C.K rewritten for efficiency.</span></div>
          </div>
        </div>
        <div class="terminal" id="terminal-container">
          <div class="terminal-header">
            <span class="terminal-dot"></span>
            <span class="terminal-title">Terminal — connected</span>
            <div class="spacer"></div>
            <span class="terminal-shell">zsh</span>
          </div>
          <div class="terminal-output" id="terminal-output"></div>
          <div class="terminal-row">
            <span class="terminal-prompt">$</span>
            <input class="terminal-input" id="terminal-input" placeholder="type a command and press Enter…">
          </div>
        </div>
      </div>
    `;
  }
}
