// esbuild build script for Keyboard
// Bundles main (Electron), preload, the React/Monaco renderer, and Monaco's web workers
// into browser/Node-compatible output. Without this, the renderer ships raw CommonJS
// `require()` calls that don't exist in the sandboxed renderer -> black screen.

const esbuild = require('esbuild');
const fs = require('fs');
const path = require('path');

const watch = process.argv.includes('--watch');
const outdir = path.join(__dirname, 'dist');
const rendererOut = path.join(outdir, 'renderer');

// Monaco ships a worker per language. We bundle each as its own browser IIFE so the
// renderer can spin them up from the local file:// directory at runtime.
const monacoWorkers = {
  'editor.worker': 'monaco-editor/esm/vs/editor/editor.worker.js',
  'ts.worker': 'monaco-editor/esm/vs/language/typescript/ts.worker.js',
  'json.worker': 'monaco-editor/esm/vs/language/json/json.worker.js',
  'css.worker': 'monaco-editor/esm/vs/language/css/css.worker.js',
  'html.worker': 'monaco-editor/esm/vs/language/html/html.worker.js',
};

/** @type {import('esbuild').BuildOptions} */
const common = {
  bundle: true,
  sourcemap: true,
  logLevel: 'info',
  define: { 'process.env.NODE_ENV': '"production"' },
};

function copyHtml() {
  fs.mkdirSync(rendererOut, { recursive: true });
  // Inject a stylesheet link for the bundled CSS (Monaco styles) if not already present.
  let html = fs.readFileSync(path.join(__dirname, 'src', 'renderer', 'index.html'), 'utf8');
  if (!html.includes('index.css')) {
    html = html.replace('</head>', '    <link rel="stylesheet" href="./index.css">\n</head>');
  }
  fs.writeFileSync(path.join(rendererOut, 'index.html'), html);
}

async function build() {
  copyHtml();

  const configs = [
    // Electron main process (Node context)
    {
      ...common,
      entryPoints: [path.join(__dirname, 'src', 'main.ts')],
      outfile: path.join(outdir, 'main.js'),
      platform: 'node',
      target: 'node20',
      format: 'cjs',
      external: ['electron', '@electron/remote', 'node-pty'],
    },
    // Preload (Node context, but runs in renderer sandbox bridge)
    {
      ...common,
      entryPoints: [path.join(__dirname, 'src', 'preload.ts')],
      outfile: path.join(outdir, 'preload.js'),
      platform: 'node',
      target: 'node20',
      format: 'cjs',
      external: ['electron'],
    },
    // Renderer (browser context): React + Monaco
    {
      ...common,
      entryPoints: [path.join(__dirname, 'src', 'renderer', 'index.tsx')],
      outfile: path.join(rendererOut, 'index.js'),
      platform: 'browser',
      target: 'chrome120',
      format: 'iife',
      loader: {
        '.ttf': 'file',
        '.woff': 'file',
        '.woff2': 'file',
        '.svg': 'file',
        '.png': 'file',
      },
    },
    // Monaco workers (browser IIFE, one file each)
    ...Object.entries(monacoWorkers).map(([name, entry]) => ({
      ...common,
      entryPoints: [require.resolve(entry)],
      outfile: path.join(rendererOut, `${name}.js`),
      platform: 'browser',
      target: 'chrome120',
      format: 'iife',
    })),
  ];

  if (watch) {
    for (const cfg of configs) {
      const ctx = await esbuild.context(cfg);
      await ctx.watch();
    }
    console.log('[build] watching for changes...');
  } else {
    await Promise.all(configs.map((cfg) => esbuild.build(cfg)));
    console.log('[build] done');
  }
}

build().catch((err) => {
  console.error(err);
  process.exit(1);
});
