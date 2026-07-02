/**
 * tck-ai — T.C.K AI Service
 *
 * Standalone Node.js entry point. Starts a Hono HTTP server that exposes
 * the Vercel AI SDK + models.dev unified API.
 *
 * Usage:
 *   npm run dev        # hot-reload via tsx
 *   npm start          # run built JS
 *   npm run build      # compile TypeScript
 *
 * Environment variables for provider API keys:
 *   ANTHROPIC_API_KEY, OPENAI_API_KEY, GOOGLE_GENERATIVE_AI_API_KEY,
 *   GROQ_API_KEY, MISTRAL_API_KEY, DEEPSEEK_API_KEY, XAI_API_KEY
 */

import { serve } from "@hono/node-server";
import app from "./server.js";

const PORT = parseInt(process.env.TCK_AI_PORT || "3002", 10);
const HOST = process.env.TCK_AI_HOST || "127.0.0.1";

serve(
  {
    fetch: app.fetch,
    port: PORT,
    hostname: HOST,
  },
  (info) => {
    console.log(`[tck-ai] AI service listening on http://${HOST}:${info.port}`);
    console.log(`[tck-ai] Endpoints:`);
    console.log(`  POST http://${HOST}:${info.port}/v1/generate`);
    console.log(`  GET  http://${HOST}:${info.port}/v1/models`);
    console.log(`  GET  http://${HOST}:${info.port}/v1/providers`);
    console.log(`  GET  http://${HOST}:${info.port}/v1/health`);
  }
);
