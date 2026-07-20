import { generateText } from "ai";
import { createOpenAI } from "@ai-sdk/openai";
import { createAnthropic } from "@ai-sdk/anthropic";
import { createGoogleGenerativeAI } from "@ai-sdk/google";
import { createGroq } from "@ai-sdk/groq";
import { createMistral } from "@ai-sdk/mistral";
import { createDeepSeek } from "@ai-sdk/deepseek";
import { createXai } from "@ai-sdk/xai";
import type { GenerateRequest, GenerateResponse } from '../../core/types.js';

const SDK_MAPPED_PROVIDERS = new Set([
  "anthropic", "openai", "google", "groq", "mistral", "deepseek", "xai",
]);

function sdkModelString(provider: string, model: string): string {
  return SDK_MAPPED_PROVIDERS.has(provider) ? `${provider}/${model}` : model;
}

function createProviderClient(provider: string, apiKey?: string, baseUrl?: string): any {
  switch (provider) {
    case "anthropic": return createAnthropic({ apiKey: apiKey || process.env.ANTHROPIC_API_KEY || "" });
    case "openai": return createOpenAI({ apiKey: apiKey || process.env.OPENAI_API_KEY || "" });
    case "google": return createGoogleGenerativeAI({ apiKey: apiKey || process.env.GOOGLE_GENERATIVE_AI_API_KEY || "" });
    case "groq": return createGroq({ apiKey: apiKey || process.env.GROQ_API_KEY || "" });
    case "mistral": return createMistral({ apiKey: apiKey || process.env.MISTRAL_API_KEY || "" });
    case "deepseek": return createDeepSeek({ apiKey: apiKey || process.env.DEEPSEEK_API_KEY || "" });
    case "xai": return createXai({ apiKey: apiKey || process.env.XAI_API_KEY || "" });
    case "openai-compatible": {
      if (!baseUrl) throw new Error("Base URL is required for openai-compatible providers");
      return createOpenAI({ apiKey: apiKey || "", baseURL: baseUrl });
    }
    default: throw new Error(`Unknown provider: ${provider}`);
  }
}

export async function generate(req: GenerateRequest): Promise<GenerateResponse> {
  const { provider, model, prompt, system, apiKey, baseUrl, temperature, maxTokens } = req;
  const providerClient = createProviderClient(provider, apiKey, baseUrl);
  const result = await generateText({
    model: providerClient(sdkModelString(provider, model)),
    prompt,
    system,
    ...(temperature !== undefined ? { temperature } : {}),
    ...(maxTokens !== undefined ? { maxTokens } : {}),
  });
  return { response: result.text, model: model };
}
