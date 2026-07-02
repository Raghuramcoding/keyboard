/**
 * models.dev integration — fetches and caches the AI model catalog.
 *
 * The models.dev API provides provider-agnostic model metadata (context window,
 * pricing, capabilities) plus provider-specific details (SDK packages, env vars).
 *
 * API endpoints:
 *   https://models.dev/api.json   — full catalog (providers + models)
 *   https://models.dev/models.json — model metadata only
 *   https://models.dev/catalog.json — combined provider endpoints + model metadata
 */

import type { ModelsDevCatalog, ModelsDevModel, ModelInfo, ProviderInfo } from "./types.js";

const MODELS_DEV_API = "https://models.dev/api.json";

// Cache the catalog for 1 hour in production, 5 minutes in dev.
const CACHE_TTL_MS =
  process.env.NODE_ENV === "production" ? 3_600_000 : 300_000;

interface CacheEntry {
  data: ModelsDevCatalog;
  expiresAt: number;
}

let cache: CacheEntry | null = null;

/** Fetch the full catalog from models.dev, with in-memory caching. */
export async function fetchCatalog(): Promise<ModelsDevCatalog> {
  const now = Date.now();
  if (cache && now < cache.expiresAt) {
    return cache.data;
  }

  const resp = await fetch(MODELS_DEV_API);
  if (!resp.ok) {
    throw new Error(
      `models.dev API returned ${resp.status}: ${resp.statusText}`
    );
  }

  const data = (await resp.json()) as ModelsDevCatalog;
  cache = { data, expiresAt: now + CACHE_TTL_MS };
  return data;
}

/** Invalidate the cached catalog so the next call fetches fresh data. */
export function invalidateCatalog(): void {
  cache = null;
}

/**
 * Return a list of providers with their models, mapped from the models.dev format
 * to our internal ProviderInfo/ModelInfo types.
 */
export async function getProviders(): Promise<ProviderInfo[]> {
  const catalog = await fetchCatalog();

  if (!catalog.providers || !catalog.models) {
    return [];
  }

  return catalog.providers
    .filter((p) => p.ai_sdk_package) // Only providers with AI SDK support
    .map((p) => {
      // Filter models that belong to this provider (models.dev model IDs often
      // have format like "provider/model-name", so we match by prefix)
      const providerModels = catalog.models
        .filter((m) => m.id.startsWith(`${p.id}/`))
        .map((m) => modelToInfo(m, p.id));

      return {
        id: p.id,
        name: p.name,
        sdkPackage: p.ai_sdk_package ?? "",
        apiKeyEnv: p.api_key_env ?? "",
        docsUrl: p.docs_url,
        baseUrl: p.base_url,
        models: providerModels,
      };
    });
}

/** Return a flat list of all models across all providers. */
export async function getModels(): Promise<ModelInfo[]> {
  const providers = await getProviders();
  return providers.flatMap((p) => p.models);
}

function modelToInfo(
  m: ModelsDevModel,
  providerId: string
): ModelInfo {
  return {
    id: m.id,
    provider: providerId,
    sdkModel: m.id,
    name: m.name,
    contextWindow: m.context_window ?? 4096,
    supportsTools: m.supports_tools ?? false,
    supportsStructuredOutput: m.supports_structured_output ?? false,
    supportsStreaming: m.supports_streaming ?? true,
    pricing: {
      input: m.pricing?.input ?? 0,
      output: m.pricing?.output ?? 0,
      cachedInput: m.pricing?.cached_input,
    },
    openWeights: m.open_weights ?? false,
    released: m.released,
  };
}
