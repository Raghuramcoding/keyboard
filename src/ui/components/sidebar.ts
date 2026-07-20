import { BaseComponent } from './base.js';

export class Sidebar extends BaseComponent {
  render(): void {
    this.element.innerHTML = `
      <div class="sidebar">
        <div>
          <div class="section-head">
            <span class="section-pip"></span>
            <span class="section-label">AI Agents</span>
          </div>
          <div class="section-list">
            <button class="agent-btn"><span class="agent-badge">CC</span>Claude Code</button>
            <button class="agent-btn"><span class="agent-badge">CX</span>Codex</button>
            <button class="agent-btn"><span class="agent-badge">OC</span>OpenCode</button>
            <button class="agent-btn"><span class="agent-badge">GE</span>Gemini CLI</button>
            <button class="agent-btn"><span class="agent-badge">AI</span>Aider</button>
          </div>
        </div>
        <div>
          <div class="section-head">
            <span class="section-pip"></span>
            <span class="section-label">Custom Commands</span>
          </div>
          <div class="section-list">
            <button class="cmd-btn"><span class="cmd-icon">▶</span><span class="cmd-label">cargo run</span></button>
            <button class="cmd-btn"><span class="cmd-icon">✓</span><span class="cmd-label">cargo test</span></button>
            <button class="cmd-btn"><span class="cmd-icon">⟳</span><span class="cmd-label">trunk serve</span></button>
          </div>
        </div>
        <div>
          <div class="section-head">
            <span class="section-pip"></span>
            <span class="section-label">Source Control</span>
          </div>
          <div class="sc-row">
            <span class="sc-connected">
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#3fd99b" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6L9 17l-5-5"/></svg>
              raghuramcoding
            </span>
            <button class="sc-disconnect">Disconnect</button>
          </div>
          <div class="repo-list">
            <button class="repo-btn"><span class="repo-name">Raghuramcoding/keyboard</span></button>
          </div>
        </div>
        <div>
          <div class="section-head">
            <span class="section-pip"></span>
            <span class="section-label">Scaffold</span>
          </div>
          <select class="scaffold-select">
            <option value="rust">Rust CLI</option>
            <option value="web">Static Web</option>
            <option value="node">Node Service</option>
            <option value="py">Python App</option>
          </select>
          <div class="scaffold-list">
            <button class="scaffold-btn">Load into editor</button>
            <input class="scaffold-input" placeholder="new-repo-name">
            <button class="scaffold-create-btn">Create GitHub repo</button>
          </div>
        </div>
      </div>
    `;
  }
}
