/**
 * Hono HTTP server for tck-ai.
 *
 * REST API:
 *   POST /v1/generate  — one-shot text generation
 *   GET  /v1/models    — list all models from models.dev
 *   GET  /v1/providers — list providers with models
 *   GET  /v1/health    — health check
 *
 * The server can run standalone (Node.js) or as a Vercel serverless function.
 */

import { Hono } from "hono";
import { cors } from "hono/cors";
import { generate } from "./generate.js";
import { getModels, getProviders } from "./models.js";
import type { GenerateRequest } from "./types.js";

const app = new Hono();

// CORS — allow the WASM UI to call from any origin (GitHub Pages, local dev)
app.use("/*", cors({ origin: "*" }));

/** Health check */
app.get("/v1/health", (c) => c.json({ status: "ok", service: "tck-ai" }));

/** Generate text via Vercel AI SDK */
app.post("/v1/generate", async (c) => {
  try {
    const body = await c.req.json<GenerateRequest>();

    // Validate required fields
    if (!body.provider || !body.model || !body.prompt) {
      return c.json(
        { error: "Missing required fields: provider, model, prompt" },
        400
      );
    }

    const result = await generate(body);
    return c.json(result);
  } catch (err) {
    console.error("[tck-ai] generate error:", err);
    const message =
      err instanceof Error ? err.message : "Internal server error";
    return c.json({ error: message }, 500);
  }
});

/** List all models from the models.dev catalog */
app.get("/v1/models", async (c) => {
  try {
    const models = await getModels();
    return c.json(models);
  } catch (err) {
    console.error("[tck-ai] models error:", err);
    const message =
      err instanceof Error ? err.message : "Failed to fetch models";
    return c.json({ error: message }, 502);
  }
});

/** List all providers with their models */
app.get("/v1/providers", async (c) => {
  try {
    const providers = await getProviders();
    return c.json(providers);
  } catch (err) {
    console.error("[tck-ai] providers error:", err);
    const message =
      err instanceof Error ? err.message : "Failed to fetch providers";
    return c.json({ error: message }, 502);
  }
});

export default app;