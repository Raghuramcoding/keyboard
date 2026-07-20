export abstract class BaseComponent {
  constructor(protected state: any, protected element: HTMLElement) {}
  abstract render(): void;

  protected createEl(tag: string, props: any = {}, children: any[] = []): HTMLElement {
    const el = document.createElement(tag);
    Object.assign(el, props);
    children.forEach(child => {
      if (typeof child === 'string') el.appendChild(document.createTextNode(child));
      else if (child) el.appendChild(child);
    });
    return el;
  }
}
