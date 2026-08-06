import type { AppState } from '../state.js';

export abstract class BaseComponent {
  constructor(protected state: AppState, protected element: HTMLElement) {}
  abstract render(): void;

  /** Escape untrusted text before it is interpolated into an HTML template. */
  protected escape(value: string): string {
    return value
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  protected createEl(tag: string, props: Record<string, unknown> = {}, children: Array<Node | string | null> = []): HTMLElement {
    const el = document.createElement(tag);
    Object.assign(el, props);
    children.forEach(child => {
      if (typeof child === 'string') el.appendChild(document.createTextNode(child));
      else if (child) el.appendChild(child);
    });
    return el;
  }
}
