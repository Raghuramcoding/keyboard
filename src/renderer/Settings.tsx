import React, { useState } from 'react';

export interface CustomButton { id: string; label: string; command: string; icon?: string; }
export interface CustomProvider { id: string; name: string; baseURL: string; apiKey: string; models: string[]; }
export interface AppSettings {
  customButtons: CustomButton[];
  background: { type: 'none' | 'color' | 'image'; color?: string; imageDataUrl?: string; opacity?: number };
  accentColor: string;
  anthropicApiKey?: string;
  terminalFontSize: number;
  workingDir?: string;
  customProviders: CustomProvider[];
}

// Quick-start presets for common OpenAI-compatible endpoints.
const PROVIDER_PRESETS: { name: string; baseURL: string }[] = [
  { name: 'OpenAI', baseURL: 'https://api.openai.com/v1' },
  { name: 'OpenRouter', baseURL: 'https://openrouter.ai/api/v1' },
  { name: 'Groq', baseURL: 'https://api.groq.com/openai/v1' },
  { name: 'Together', baseURL: 'https://api.together.xyz/v1' },
  { name: 'DeepSeek', baseURL: 'https://api.deepseek.com/v1' },
  { name: 'Mistral', baseURL: 'https://api.mistral.ai/v1' },
  { name: 'LM Studio (local)', baseURL: 'http://localhost:1234/v1' },
  { name: 'llama.cpp (local)', baseURL: 'http://localhost:8080/v1' },
];

interface Props {
  settings: AppSettings;
  onClose: () => void;
  onSave: (s: AppSettings) => void;
}

const ACCENTS = ['#0e639c', '#6f42c1', '#d4a574', '#e67e22', '#2ea043', '#d4377d', '#3794ff'];

const SettingsModal: React.FC<Props> = ({ settings, onClose, onSave }) => {
  const [draft, setDraft] = useState<AppSettings>(JSON.parse(JSON.stringify(settings)));
  const [newLabel, setNewLabel] = useState('');
  const [newCmd, setNewCmd] = useState('');
  const [newIcon, setNewIcon] = useState('▶️');

  const update = (patch: Partial<AppSettings>) => setDraft((d) => ({ ...d, ...patch }));
  const updateBg = (patch: Partial<AppSettings['background']>) => setDraft((d) => ({ ...d, background: { ...d.background, ...patch } }));

  const addButton = () => {
    if (!newLabel.trim() || !newCmd.trim()) return;
    update({ customButtons: [...draft.customButtons, { id: `cb-${Date.now()}`, label: newLabel.trim(), command: newCmd.trim(), icon: newIcon.trim() || '▶️' }] });
    setNewLabel(''); setNewCmd(''); setNewIcon('▶️');
  };
  const removeButton = (id: string) => update({ customButtons: draft.customButtons.filter((b) => b.id !== id) });

  const pickImage = async () => {
    const r = await window.electronAPI.pickImage();
    if (!r.canceled && r.dataUrl) updateBg({ type: 'image', imageDataUrl: r.dataUrl });
  };
  const pickDir = async () => {
    const r = await window.electronAPI.pickDirectory();
    if (!r.canceled && r.path) update({ workingDir: r.path });
  };

  return (
    <div style={s.overlay} onClick={onClose}>
      <div style={s.modal} onClick={(e) => e.stopPropagation()}>
        <div style={s.header}>
          <h3 style={{ margin: 0, color: '#e0e0e0' }}>⚙️ Settings</h3>
          <button onClick={onClose} style={s.x}>×</button>
        </div>

        <div style={s.scroll}>
          {/* Working directory */}
          <Section title="Working Directory">
            <div style={{ display: 'flex', gap: 6 }}>
              <input value={draft.workingDir || ''} onChange={(e) => update({ workingDir: e.target.value })} placeholder="Project folder for terminal & git" style={s.input} />
              <button onClick={pickDir} style={s.btn}>📁 Browse</button>
            </div>
          </Section>

          {/* Custom command buttons */}
          <Section title="Custom Command Buttons">
            <div style={{ fontSize: 11, color: '#888', marginBottom: 8 }}>Buttons appear in the toolbar. Clicking one opens the terminal and runs its command.</div>
            {draft.customButtons.length === 0 && <div style={{ fontSize: 12, color: '#666', marginBottom: 8 }}>No custom buttons yet.</div>}
            {draft.customButtons.map((b) => (
              <div key={b.id} style={s.row}>
                <span style={{ width: 24, textAlign: 'center' }}>{b.icon}</span>
                <span style={{ flex: 1, fontSize: 12 }}>{b.label}</span>
                <code style={{ flex: 2, fontSize: 11, color: '#9cdcfe', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{b.command}</code>
                <button onClick={() => removeButton(b.id)} style={s.del}>🗑️</button>
              </div>
            ))}
            <div style={{ display: 'flex', gap: 6, marginTop: 8 }}>
              <input value={newIcon} onChange={(e) => setNewIcon(e.target.value)} style={{ ...s.input, width: 44, textAlign: 'center' }} title="Icon/emoji" />
              <input value={newLabel} onChange={(e) => setNewLabel(e.target.value)} placeholder="Label" style={{ ...s.input, flex: 1 }} />
              <input value={newCmd} onChange={(e) => setNewCmd(e.target.value)} placeholder="Command, e.g. npm run dev" style={{ ...s.input, flex: 2 }} onKeyDown={(e) => e.key === 'Enter' && addButton()} />
              <button onClick={addButton} style={s.btnAccent}>＋ Add</button>
            </div>
          </Section>

          {/* OpenAI-compatible providers */}
          <Section title="AI Providers (OpenAI-compatible)">
            <div style={{ fontSize: 11, color: '#888', marginBottom: 8 }}>
              Connect any OpenAI-compatible API (OpenAI, OpenRouter, Groq, local LM Studio, …) via the Vercel AI SDK.
              Browse models from <span style={{ color: '#9cdcfe' }}>models.dev</span>.
            </div>
            <ProvidersSection providers={draft.customProviders} onChange={(cp) => update({ customProviders: cp })} />
          </Section>

          {/* Background */}
          <Section title="Background">
            <div style={{ display: 'flex', gap: 6, marginBottom: 8 }}>
              {(['none', 'color', 'image'] as const).map((t) => (
                <button key={t} onClick={() => updateBg({ type: t })} style={{ ...s.pill, ...(draft.background.type === t ? s.pillActive : {}) }}>{t}</button>
              ))}
            </div>
            {draft.background.type === 'color' && (
              <input type="color" value={draft.background.color || '#1e1e1e'} onChange={(e) => updateBg({ color: e.target.value })} style={{ width: 60, height: 32, background: 'none', border: '1px solid #3c3c3c', borderRadius: 4 }} />
            )}
            {draft.background.type === 'image' && (
              <div>
                <button onClick={pickImage} style={s.btn}>🖼️ Choose Image…</button>
                {draft.background.imageDataUrl && <img src={draft.background.imageDataUrl} alt="bg" style={{ display: 'block', marginTop: 8, maxHeight: 80, borderRadius: 4, border: '1px solid #3c3c3c' }} />}
              </div>
            )}
            {draft.background.type !== 'none' && (
              <div style={{ marginTop: 10 }}>
                <label style={s.lbl}>Panel opacity: {Math.round((draft.background.opacity ?? 0.25) * 100)}%</label>
                <input type="range" min="0" max="0.9" step="0.05" value={draft.background.opacity ?? 0.25} onChange={(e) => updateBg({ opacity: parseFloat(e.target.value) })} style={{ width: '100%' }} />
                <div style={{ fontSize: 10, color: '#666' }}>Higher = more of the background shows through the panels.</div>
              </div>
            )}
          </Section>

          {/* Accent */}
          <Section title="Accent Color">
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              {ACCENTS.map((c) => (
                <div key={c} onClick={() => update({ accentColor: c })} title={c}
                  style={{ width: 24, height: 24, borderRadius: '50%', background: c, cursor: 'pointer', border: draft.accentColor === c ? '2px solid #fff' : '2px solid transparent' }} />
              ))}
              <input type="color" value={draft.accentColor} onChange={(e) => update({ accentColor: e.target.value })} style={{ width: 36, height: 28, background: 'none', border: '1px solid #3c3c3c', borderRadius: 4 }} />
            </div>
          </Section>

          {/* Terminal font size */}
          <Section title="Terminal Font Size">
            <input type="range" min="10" max="22" step="1" value={draft.terminalFontSize} onChange={(e) => update({ terminalFontSize: parseInt(e.target.value) })} style={{ width: '100%' }} />
            <div style={{ fontSize: 12, color: '#888' }}>{draft.terminalFontSize}px</div>
          </Section>

          {/* API key */}
          <Section title="Anthropic API Key">
            <input type="password" value={draft.anthropicApiKey || ''} onChange={(e) => update({ anthropicApiKey: e.target.value })} placeholder="sk-ant-… (enables Claude models)" style={s.input} />
            <div style={{ fontSize: 10, color: '#666', marginTop: 4 }}>Stored locally in your settings file. Falls back to the ANTHROPIC_API_KEY env var.</div>
          </Section>
        </div>

        <div style={s.footer}>
          <button onClick={onClose} style={s.btn}>Cancel</button>
          <button onClick={() => onSave(draft)} style={s.btnAccent}>💾 Save</button>
        </div>
      </div>
    </div>
  );
};

const Section: React.FC<{ title: string; children: React.ReactNode }> = ({ title, children }) => (
  <div style={{ marginBottom: 18 }}>
    <div style={s.lbl}>{title}</div>
    {children}
  </div>
);

// ─── OpenAI-compatible provider manager ──────────────────────────────────────
const ProvidersSection: React.FC<{ providers: CustomProvider[]; onChange: (p: CustomProvider[]) => void }> = ({ providers, onChange }) => {
  const [editing, setEditing] = useState<CustomProvider | null>(null);
  const [modelInput, setModelInput] = useState('');
  const [fetched, setFetched] = useState<string[]>([]);
  const [status, setStatus] = useState('');
  const [catalog, setCatalog] = useState<any>(null);
  const [showCatalog, setShowCatalog] = useState(false);
  const [filter, setFilter] = useState('');

  const setEd = (patch: Partial<CustomProvider>) => setEditing((e) => (e ? { ...e, ...patch } : e));
  const startNew = () => { setEditing({ id: 'cp-' + Date.now(), name: '', baseURL: '', apiKey: '', models: [] }); setFetched([]); setStatus(''); setShowCatalog(false); };
  const startEdit = (p: CustomProvider) => { setEditing(JSON.parse(JSON.stringify(p))); setFetched([]); setStatus(''); setShowCatalog(false); };
  const cancel = () => { setEditing(null); setShowCatalog(false); setFetched([]); };
  const remove = (id: string) => onChange(providers.filter((p) => p.id !== id));
  const save = () => {
    if (!editing) return;
    if (!editing.name.trim() || !editing.baseURL.trim()) { alert('Name and Base URL are required.'); return; }
    const exists = providers.some((p) => p.id === editing.id);
    onChange(exists ? providers.map((p) => (p.id === editing.id ? editing : p)) : [...providers, editing]);
    setEditing(null); setShowCatalog(false);
  };

  const addModel = (m: string) => { const v = m.trim(); if (!v || !editing) return; if (!editing.models.includes(v)) setEd({ models: [...editing.models, v] }); };
  const removeModel = (m: string) => editing && setEd({ models: editing.models.filter((x) => x !== m) });

  const fetchModels = async () => {
    if (!editing) return;
    setStatus('Fetching models from endpoint…'); setFetched([]);
    const r = await window.electronAPI.listProviderModels(editing.baseURL, editing.apiKey);
    if (r.error) setStatus(`Error: ${r.error}`);
    else { setFetched(r.models || []); setStatus(`Found ${r.models?.length || 0} models — click to add`); }
  };

  const loadCatalog = async () => {
    setShowCatalog((v) => !v);
    if (catalog) return;
    setStatus('Loading models.dev catalog…');
    const r = await window.electronAPI.getModelCatalog();
    if (r.error) setStatus(`Error: ${r.error}`);
    else { setCatalog(r.data); setStatus(''); }
  };

  // Flatten models.dev catalog into searchable entries.
  const catalogEntries: { provider: string; providerName: string; api?: string; model: string }[] = [];
  if (catalog && typeof catalog === 'object') {
    for (const key of Object.keys(catalog)) {
      const prov = catalog[key] || {};
      const models = prov.models || {};
      for (const mk of Object.keys(models)) {
        catalogEntries.push({ provider: key, providerName: prov.name || key, api: prov.api, model: models[mk]?.id || mk });
      }
    }
  }
  const filtered = filter
    ? catalogEntries.filter((e) => (e.providerName + ' ' + e.model).toLowerCase().includes(filter.toLowerCase())).slice(0, 200)
    : catalogEntries.slice(0, 60);

  return (
    <div>
      {providers.length === 0 && !editing && <div style={{ fontSize: 12, color: '#666', marginBottom: 8 }}>No providers configured.</div>}
      {providers.map((p) => (
        <div key={p.id} style={s.row}>
          <span style={{ flex: 1, fontSize: 12, fontWeight: 600 }}>{p.name}</span>
          <code style={{ flex: 2, fontSize: 11, color: '#9cdcfe', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{p.baseURL}</code>
          <span style={{ fontSize: 11, color: '#888' }}>{p.models.length} models</span>
          <button onClick={() => startEdit(p)} style={s.del} title="Edit">✏️</button>
          <button onClick={() => remove(p.id)} style={s.del} title="Remove">🗑️</button>
        </div>
      ))}

      {!editing && <button onClick={startNew} style={{ ...s.btnAccent, marginTop: 8 }}>＋ Add Provider</button>}

      {editing && (
        <div style={s.editor}>
          {/* Presets */}
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginBottom: 8 }}>
            {PROVIDER_PRESETS.map((p) => (
              <button key={p.name} onClick={() => setEd({ name: editing.name || p.name, baseURL: p.baseURL })} style={s.preset}>{p.name}</button>
            ))}
          </div>
          <div style={{ display: 'flex', gap: 6, marginBottom: 6 }}>
            <input value={editing.name} onChange={(e) => setEd({ name: e.target.value })} placeholder="Name (e.g. OpenRouter)" style={{ ...s.input, flex: 1 }} />
          </div>
          <input value={editing.baseURL} onChange={(e) => setEd({ baseURL: e.target.value })} placeholder="Base URL (e.g. https://openrouter.ai/api/v1)" style={{ ...s.input, width: '100%', marginBottom: 6 }} />
          <input type="password" value={editing.apiKey} onChange={(e) => setEd({ apiKey: e.target.value })} placeholder="API key" style={{ ...s.input, width: '100%', marginBottom: 8 }} />

          {/* Selected models */}
          <div style={s.lbl}>Models</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginBottom: 6 }}>
            {editing.models.length === 0 && <span style={{ fontSize: 11, color: '#666' }}>None yet — add manually, fetch, or browse models.dev.</span>}
            {editing.models.map((m) => (
              <span key={m} style={s.chip}>{m}<span onClick={() => removeModel(m)} style={{ cursor: 'pointer', color: '#e06c75' }}>×</span></span>
            ))}
          </div>
          <div style={{ display: 'flex', gap: 6, marginBottom: 6 }}>
            <input value={modelInput} onChange={(e) => setModelInput(e.target.value)} placeholder="model id" style={{ ...s.input, flex: 1 }} onKeyDown={(e) => { if (e.key === 'Enter') { addModel(modelInput); setModelInput(''); } }} />
            <button onClick={() => { addModel(modelInput); setModelInput(''); }} style={s.btn}>Add</button>
            <button onClick={fetchModels} style={s.btn} title="GET {baseURL}/models">🔍 Fetch</button>
            <button onClick={loadCatalog} style={s.btn}>📚 models.dev</button>
          </div>
          {status && <div style={{ fontSize: 11, color: '#888', marginBottom: 6 }}>{status}</div>}

          {/* Fetched-from-endpoint list */}
          {fetched.length > 0 && (
            <div style={s.modelList}>
              {fetched.map((m) => (
                <div key={m} onClick={() => addModel(m)} style={s.modelOption} title="Add">{m}</div>
              ))}
            </div>
          )}

          {/* models.dev catalog browser */}
          {showCatalog && (
            <div style={{ marginTop: 6 }}>
              <input value={filter} onChange={(e) => setFilter(e.target.value)} placeholder="Search models.dev…" style={{ ...s.input, width: '100%', marginBottom: 4 }} />
              <div style={s.modelList}>
                {filtered.map((e, i) => (
                  <div key={e.provider + e.model + i} onClick={() => { addModel(e.model); if (!editing.baseURL && e.api) setEd({ baseURL: e.api, name: editing.name || e.providerName }); }} style={s.modelOption} title={`Add ${e.model}`}>
                    <span style={{ color: '#888' }}>{e.providerName}</span> / {e.model}
                  </div>
                ))}
                {filtered.length === 0 && <div style={{ fontSize: 11, color: '#666', padding: 6 }}>No matches.</div>}
              </div>
            </div>
          )}

          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 6, marginTop: 10 }}>
            <button onClick={cancel} style={s.btn}>Cancel</button>
            <button onClick={save} style={s.btnAccent}>Save Provider</button>
          </div>
        </div>
      )}
    </div>
  );
};

const s: Record<string, React.CSSProperties> = {
  overlay: { position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.6)', display: 'flex', justifyContent: 'center', alignItems: 'center', zIndex: 2000 },
  modal: { background: '#252526', border: '1px solid #3c3c3c', borderRadius: 10, width: 600, maxHeight: '85vh', display: 'flex', flexDirection: 'column' },
  header: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '14px 18px', borderBottom: '1px solid #3c3c3c' },
  x: { background: 'none', border: 'none', color: '#888', fontSize: 22, cursor: 'pointer' },
  scroll: { padding: '14px 18px', overflowY: 'auto' },
  footer: { display: 'flex', justifyContent: 'flex-end', gap: 8, padding: '12px 18px', borderTop: '1px solid #3c3c3c' },
  lbl: { fontSize: 11, fontWeight: 600, textTransform: 'uppercase', color: '#888', letterSpacing: '0.5px', marginBottom: 8 },
  input: { padding: '7px 10px', fontSize: 12, background: '#1e1e1e', color: '#d4d4d4', border: '1px solid #3c3c3c', borderRadius: 5, outline: 'none' },
  row: { display: 'flex', alignItems: 'center', gap: 8, padding: '5px 6px', borderBottom: '1px solid #2d2d2d' },
  btn: { padding: '7px 12px', fontSize: 12, cursor: 'pointer', background: '#333', color: '#d4d4d4', border: '1px solid #4c4c4c', borderRadius: 5 },
  btnAccent: { padding: '7px 14px', fontSize: 12, fontWeight: 600, cursor: 'pointer', background: 'var(--accent)', color: '#fff', border: 'none', borderRadius: 5 },
  del: { background: 'none', border: 'none', cursor: 'pointer', fontSize: 13 },
  pill: { padding: '5px 14px', fontSize: 12, textTransform: 'capitalize', cursor: 'pointer', background: '#1e1e1e', color: '#ccc', border: '1px solid #3c3c3c', borderRadius: 14 },
  pillActive: { background: 'var(--accent)', color: '#fff', borderColor: 'transparent' },
  editor: { marginTop: 10, padding: 12, background: '#1e1e1e', border: '1px solid #3c3c3c', borderRadius: 8 },
  preset: { padding: '3px 8px', fontSize: 11, cursor: 'pointer', background: '#2d2d2d', color: '#ccc', border: '1px solid #3c3c3c', borderRadius: 4 },
  chip: { display: 'inline-flex', alignItems: 'center', gap: 6, padding: '3px 8px', fontSize: 11, background: '#2d2d2d', color: '#d4d4d4', border: '1px solid #3c3c3c', borderRadius: 12 },
  modelList: { maxHeight: 160, overflowY: 'auto', border: '1px solid #3c3c3c', borderRadius: 5, background: '#161616' },
  modelOption: { padding: '5px 8px', fontSize: 11, cursor: 'pointer', borderBottom: '1px solid #2a2a2a', color: '#d4d4d4' },
};

export default SettingsModal;
