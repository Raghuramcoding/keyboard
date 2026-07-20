import { BaseComponent } from './base.js';

export class TitleBar extends BaseComponent {
  render(): void {
    this.element.innerHTML = `
      <div class="titlebar">
        <div class="brand">
          <div class="brand-mark">
            <svg viewBox="0 0 26 17" width="17" height="11" aria-hidden="true">
              <g fill="#fff">
                <circle cx="8.5" cy="9.5" r="5.2"/><circle cx="15" cy="7" r="6.4"/><circle cx="19.5" cy="10.5" r="4.4"/><rect x="5" y="9.5" width="16" height="5.5" rx="2.7"/>
              </g>
            </svg>
          </div>
          <div class="brand-text">
            <span class="brand-name">T.C.K</span>
            <span class="brand-sub">TalentCloud Keyboard</span>
          </div>
        </div>
        <div class="spacer"></div>
        <select class="tb-select" id="providerSelect">
          <option value="ollama">Ollama</option>
          <option value="claude" selected>Claude</option>
          <option value="openrouter">OpenRouter</option>
          <option value="openai">OpenAI</option>
        </select>
        <input class="tb-model" id="modelInput" value="${this.state.model}" placeholder="model…">
        <button class="tb-icon-btn" id="themeBtn">☀</button>
        <button class="tb-settings-btn" id="settingsBtn">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
          </svg>
          Settings
        </button>
      </div>
    `;

    this.element.querySelector('#providerSelect')?.addEventListener('change', (e: any) => {
      this.state.provider = e.target.value;
    });
    this.element.querySelector('#modelInput')?.addEventListener('input', (e: any) => {
      this.state.model = e.target.value;
    });
    this.element.querySelector('#settingsBtn')?.addEventListener('click', () => {
      this.state.showSettings = true;
      window.dispatchEvent(new CustomEvent('app-state-changed'));
    });
  }
}
