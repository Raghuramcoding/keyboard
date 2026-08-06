import { Settings, InstanceInfo } from '../core/types.js';

const INSTANCES_KEY = 'tck.instances';
const SETTINGS_KEY = 'tck.settings';

export class AppState {
  instances: InstanceInfo[] = [];
  settings: Settings = { providers: [], commands: [], claude_api_key: '', ollama_host: '', background: '' };
  provider = 'claude';
  model = 'claude-sonnet-4-5';
  showSettings = false;

  persistInstances() {
    localStorage.setItem(INSTANCES_KEY, JSON.stringify(this.instances));
  }

  loadInstances() {
    this.instances = readJson<InstanceInfo[]>(INSTANCES_KEY) ?? [];
  }

  persistSettings() {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(this.settings));
  }

  loadSettings() {
    const stored = readJson<Partial<Settings>>(SETTINGS_KEY);
    if (stored) this.settings = { ...this.settings, ...stored };
  }
}

function readJson<T>(key: string): T | null {
  const raw = localStorage.getItem(key);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as T;
  } catch {
    // Corrupt entry — drop it rather than breaking startup.
    localStorage.removeItem(key);
    return null;
  }
}

export const state = new AppState();
