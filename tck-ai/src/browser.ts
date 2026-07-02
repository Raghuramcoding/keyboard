/**
 * Browser entry point for tck-ai.
 *
 * Bundled by esbuild into tck-ai.js and loaded via <script> tag on GitHub Pages.
 * Exposes a global `TckAI` object that the WASM module can call via JS interop.
 *
 * Uses the Vercel AI SDK's browser-compatible providers directly in the browser
 * (no server needed for GitHub Pages mode).
 */

import { generateText } from "ai";
import { createOpenAI } from "@ai-sdk/openai";
import { createAnthropic } from "@ai-sdk/anthropic";
import { createGoogleGenerativeAI } from "@ai-sdk/google";

// ---- Types mirroring the server API ----
interface GenerateRequest {
  provider: string;
  model: string;
  prompt: string;
  system?: string;
  apiKey?: string;
  baseUrl?: string;
  temperature?: number;
  maxTokens?: number;
}

interface GenerateResponse {
  response: string;
  model: string;
  usage?: {
    promptTokens: number;
    completionTokens: number;
    totalTokens: number;
  };
}

// SDK-mapped providers
const SDK_PROVIDERS = new Set(["anthropic", "openai", "google"]);

function createClient(provider: string, apiKey?: string, baseUrl?: string): any {
  switch (provider) {
    case "anthropic":
      return createAnthropic({
        apiKey: apiKey || "",
      });
    case "openai":
      return createOpenAI({
        apiKey: apiKey || "",
        baseURL: baseUrl,
      });
    case "google":
      return createGoogleGenerativeAI({
        apiKey: apiKey || "",
      });
    default:
      return createOpenAI({
        apiKey: apiKey || "",
        baseURL: baseUrl,
      });
  }
}

/** One-shot text generation from the browser. */
async function browserGenerate(
  req: GenerateRequest
): Promise<GenerateResponse> {
  const { provider, model, prompt, system, apiKey, baseUrl, temperature, maxTokens } =
    req;

  const client = createClient(provider, apiKey, baseUrl);

  const modelStr = SDK_PROVIDERS.has(provider)
    ? `${provider}/${model}`
    : model;

  const result = await generateText({
    model: client(modelStr),
    prompt,
    system: system,
    ...(temperature !== undefined ? { temperature } : {}),
    ...(maxTokens !== undefined ? { maxTokens } : {}),
  });

  return {
    response: result.text,
    model: model,
    usage: result.usage
      ? {
          promptTokens: result.usage.inputTokens ?? 0,
          completionTokens: result.usage.outputTokens ?? 0,
          totalTokens: (result.usage.inputTokens ?? 0) + (result.usage.outputTokens ?? 0),
        }
      : undefined,
  };
}

// ---- Global API for WASM interop ----
const TckAI = {
  generate: browserGenerate,
};

// Make it globally available
if (typeof window !== "undefined") {
  (window as any).TckAI = TckAI;
}

export { TckAI, browserGenerate as generate };
