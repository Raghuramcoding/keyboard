import { BaseComponent } from './base.js';

export class SettingsModal extends BaseComponent {
  render(): void {
    this.element.innerHTML = `
      <div class="modal-backdrop ${this.state.showSettings ? 'open' : ''}" id="settingsModal">
        <div class="modal" id="modalBox">
          <div class="modal-header">
            <span class="modal-icon">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
              </svg>
            </span>
            <span class="modal-title">Settings</span>
            <div class="spacer"></div>
            <button class="modal-close" id="modalClose">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6L6 18"/><path d="M6 6l12 12"/></svg>
            </button>
          </div>
          <div class="modal-body">
            <div>
              <div class="field-label">Claude API key</div>
              <input type="password" class="field-input" value="${this.state.settings.claude_api_key}">
            </div>
            <div>
              <div class="field-label">Ollama host</div>
              <input class="field-input" value="${this.state.settings.ollama_host}">
            </div>
          </div>
          <div class="modal-footer">
            <button class="btn-cancel" id="modalCancel">Cancel</button>
            <button class="btn-save" id="modalSave">Save</button>
          </div>
        </div>
      </div>
    `;

    this.element.querySelector('#modalClose')?.addEventListener('click', () => {
      this.state.showSettings = false;
      this.render();
    });
    this.element.querySelector('#modalCancel')?.addEventListener('click', () => {
      this.state.showSettings = false;
      this.render();
    });
    this.element.querySelector('#modalSave')?.addEventListener('click', () => {
      this.state.showSettings = false;
      this.render();
    });
  }
}
