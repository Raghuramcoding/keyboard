import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { serveStatic } from '@hono/node-server/serve-static';
import { serve } from '@hono/node-server';
import { WebSocketServer } from 'ws';
import { aiRoutes } from './routes/ai.js';
import { instanceRoutes } from './routes/instances.js';
import { githubRoutes } from './routes/github.js';
import { ptyService } from './services/pty.js';

const PORT = Number(process.env.PORT ?? 3000);
const DIST = process.env.TCK_DIST ?? 'dist';

const app = new Hono();
app.use('/*', cors());

app.get('/health', (c) => c.json({ status: 'ok' }));

app.route('/api/ai', aiRoutes);
app.route('/api/instances', instanceRoutes);
app.route('/api/github', githubRoutes);

// Serve the built UI (falling back to index.html for hash-routed views).
app.use('/*', serveStatic({ root: DIST }));
app.get('*', serveStatic({ path: `${DIST}/index.html` }));

const server = serve({ fetch: app.fetch, port: PORT }, () => {
  console.log(`T.C.K server listening on http://localhost:${PORT}`);
});

const wss = new WebSocketServer({ noServer: true });

server.on('upgrade', (req, socket, head) => {
  const url = new URL(req.url ?? '/', `http://${req.headers.host ?? 'localhost'}`);
  const match = /^\/ws\/pty\/(.+)$/.exec(url.pathname);
  if (!match) {
    socket.destroy();
    return;
  }
  wss.handleUpgrade(req, socket, head, (ws) => {
    ptyService.handleTerminalWs(ws, match[1]);
  });
});
