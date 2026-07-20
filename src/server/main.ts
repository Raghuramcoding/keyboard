import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { serve } from '@hono/node-server';
import { aiRoutes } from './routes/ai.js';
import { instanceRoutes } from './routes/instances.js';
import { githubRoutes } from './routes/github.js';

const app = new Hono();
app.use('/*', cors());

app.get('/', (c) => c.text('T.C.K Server Running'));

app.route('/api/ai', aiRoutes);
app.route('/api/instances', instanceRoutes);
app.route('/api/github', githubRoutes);

serve({ fetch: app.fetch, port: 3000 }, () => {
  console.log('T.C.K Server listening on http://localhost:3000');
});
