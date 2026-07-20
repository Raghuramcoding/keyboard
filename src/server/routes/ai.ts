import { Hono } from 'hono';
import { generate } from '../services/ai.js';
import { getModels, getProviders } from '../services/models.js';
import type { GenerateRequest } from '../../core/types.js';

export const aiRoutes = new Hono();

aiRoutes.post('/generate', async (c) => {
  try {
    const body = await c.req.json<GenerateRequest>();
    if (!body.provider || !body.model || !body.prompt) {
      return c.json({ error: "Missing required fields: provider, model, prompt" }, 400);
    }
    const result = await generate(body);
    return c.json(result);
  } catch (err) {
    console.error("[tck-server] generate error:", err);
    return c.json({ error: err instanceof Error ? err.message : "Internal server error" }, 500);
  }
});

aiRoutes.get('/models', async (c) => {
  try {
    const models = await getModels();
    return c.json(models);
  } catch (err) {
    return c.json({ error: "Failed to fetch models" }, 502);
  }
});

aiRoutes.get('/providers', async (c) => {
  try {
    const providers = await getProviders();
    return c.json(providers);
  } catch (err) {
    return c.json({ error: "Failed to fetch providers" }, 502);
  }
});
