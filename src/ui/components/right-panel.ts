import { BaseComponent } from './base.js';

export class RightPanel extends BaseComponent {
  render(): void {
    this.element.innerHTML = `
      <div class="chat-panel">
        <div class="chat-header">
          <span class="chat-icon">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 8V4H8"/><rect x="4" y="8" width="16" height="12" rx="2"/>
              <path d="M2 14h2"/><path d="M20 14h2"/><path d="M15 13v2"/><path d="M9 13v2"/>
            </svg>
          </span>
          <span class="chat-title">AI Chat</span>
          <div class="spacer"></div>
          <span class="model-pill">${this.state.model}</span>
        </div>
        <div class="chat-messages" id="chat-messages">
          <div class="msg-ai">
            <div class="msg-who ai">Assistant</div>
            <div class="msg-bubble-ai">Welcome to T.C.K. How can I help you today?</div>
          </div>
        </div>
        <div class="chat-input-area">
          <textarea class="chat-textarea" id="chat-input" placeholder="Message the model…"></textarea>
          <button class="chat-send-btn" id="chat-send">
            Send
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M13 6l6 6-6 6"/></svg>
          </button>
        </div>
      </div>
    `;

    this.element.querySelector('#chat-send')?.addEventListener('click', async () => {
      const input = this.element.querySelector('#chat-input') as HTMLTextAreaElement;
      const text = input.value.trim();
      if (!text) return;

      const messages = this.element.querySelector('#chat-messages');
      messages.innerHTML += `
        <div class="msg-user">
          <div class="msg-who">You</div>
          <div class="msg-bubble-user">${text}</div>
        </div>
      `;
      input.value = '';

      // Call AI API
      const resp = await fetch('/api/ai/generate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          provider: this.state.provider,
          model: this.state.model,
          prompt: text,
        }),
      });
      const data = await resp.json();
      messages.innerHTML += `
        <div class="msg-ai">
          <div class="msg-who ai">Assistant</div>
          <div class="msg-bubble-ai">${data.response || data.error}</div>
        </div>
      `;
    });
  }
}
