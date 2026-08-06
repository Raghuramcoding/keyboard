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
          <span class="model-pill">${this.escape(this.state.model)}</span>
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

    const input = this.element.querySelector<HTMLTextAreaElement>('#chat-input');
    const sendBtn = this.element.querySelector<HTMLButtonElement>('#chat-send');
    const messages = this.element.querySelector<HTMLElement>('#chat-messages');
    if (!input || !sendBtn || !messages) return;

    const send = async () => {
      const text = input.value.trim();
      if (!text || sendBtn.disabled) return;

      this.appendMessage(messages, 'You', text, false);
      input.value = '';
      sendBtn.disabled = true;

      try {
        const resp = await fetch('/api/ai/generate', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            provider: this.state.provider,
            model: this.state.model,
            prompt: text,
            api_key: this.state.provider === 'claude'
              ? this.state.settings.claude_api_key || undefined
              : undefined,
            base_url: this.state.provider === 'ollama'
              ? this.state.settings.ollama_host || undefined
              : undefined,
          }),
        });
        const data = await resp.json() as { response?: string; error?: string };
        this.appendMessage(messages, 'Assistant', data.response ?? data.error ?? 'No response', true);
      } catch (err) {
        const reason = err instanceof Error ? err.message : 'Request failed';
        this.appendMessage(messages, 'Assistant', `Request failed: ${reason}`, true);
      } finally {
        sendBtn.disabled = false;
      }
    };

    sendBtn.addEventListener('click', () => void send());
    input.addEventListener('keydown', (e: KeyboardEvent) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        void send();
      }
    });
  }

  /** Append a chat bubble. Text is set via textContent so it is never parsed as HTML. */
  private appendMessage(container: HTMLElement, who: string, text: string, isAi: boolean): void {
    const wrapper = document.createElement('div');
    wrapper.className = isAi ? 'msg-ai' : 'msg-user';

    const label = document.createElement('div');
    label.className = isAi ? 'msg-who ai' : 'msg-who';
    label.textContent = who;

    const bubble = document.createElement('div');
    bubble.className = isAi ? 'msg-bubble-ai' : 'msg-bubble-user';
    bubble.textContent = text;

    wrapper.append(label, bubble);
    container.appendChild(wrapper);
    container.scrollTop = container.scrollHeight;
  }
}
