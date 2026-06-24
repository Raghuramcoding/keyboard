import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';

// Tell Monaco where to load its web workers from. The workers are bundled as
// sibling files next to index.js (see build.js), and the renderer is served from
// the same file:// directory, so relative URLs resolve correctly.
(self as any).MonacoEnvironment = {
  getWorkerUrl(_moduleId: string, label: string): string {
    if (label === 'typescript' || label === 'javascript') return './ts.worker.js';
    if (label === 'json') return './json.worker.js';
    if (label === 'css' || label === 'scss' || label === 'less') return './css.worker.js';
    if (label === 'html' || label === 'handlebars' || label === 'razor') return './html.worker.js';
    return './editor.worker.js';
  },
};

const root = ReactDOM.createRoot(
  document.getElementById('root') as HTMLElement
);
root.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
