import { Settings, InstanceInfo } from '../core/types.js';

export class AppState {
  instances: InstanceInfo[] = [];
  settings: Settings = { providers: [], commands: [], claude_api_key: '', ollama_host: '', background: '' };
  provider = 'claude';
  model = 'claude-opus-4-8';
  showSettings = false;

  persistInstances() {
    localStorage.setItem('tck.instances', JSON.stringify(this.instances));
  }
  loadInstances() {
    const data = localStorage.getItem('tck.instances');
    if (data) this.instances = JSON.parse(data);
  }
}
export const state = new AppState();
