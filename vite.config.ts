import { defineConfig } from 'vite';

// The UI is a plain TypeScript SPA; `index.html` at the repo root is the entry.
export default defineConfig({
  root: '.',
  build: { outDir: 'dist', emptyOutDir: true },
  server: {
    port: 8080,
    proxy: {
      '/api': 'http://127.0.0.1:3000',
      '/ws': { target: 'ws://127.0.0.1:3000', ws: true },
    },
  },
});
