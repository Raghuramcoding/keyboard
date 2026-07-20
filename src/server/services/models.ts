export interface ModelsDevCatalog {
  providers: Array<{ id: string; name: string; ai_sdk_package?: string; api_key_env?: string; docs_url?: string; base_url?: string; }>;
  models: Array<{ id: string; name: string; context_window?: number; supports_tools?: boolean; supports_structured_output?: boolean; supports_streaming?: boolean; pricing?: { input?: number; output?: number; cached_input?: number; }; open_weights?: boolean; released?: string; }>;
}

export interface ModelInfo {
  id: string; provider: string; sdkModel: string; name: string; contextWindow: number; supportsTools: boolean; supportsStructuredOutput: boolean; supportsStreaming: boolean; pricing: { input: number; output: number; cachedInput?: number; }; openWeights: boolean; released?: string;
}

export interface ProviderInfo {
  id: string; name: string; sdkPackage: string; apiKeyEnv: string; docsUrl?: string; baseUrl?: string; models: ModelInfo[];
}

const MODELS_DEV_API = "https://models.dev/api.json";
const CACHE_TTL_MS = process.env.NODE_ENV === "production" ? 3_600_000 : 300_000;
let cache: { data: ModelsDevCatalog; expiresAt: number } | null = null;

export async function fetchCatalog(): Promise<ModelsDevCatalog> {
  const now = Date.now();
  if (cache && now < cache.expiresAt) return cache.data;
  const resp = await fetch(MODELS_DEV_API);
  if (!resp.ok) throw new Error(`models.dev API returned ${resp.status}`);
  const data = await resp.json() as ModelsDev laCatalog;
  cache = { data, expiresAt: now + CACHE_TTL_MS };
  return data;
}

export async function getProviders(): Promise<ProviderInfo[]> {
  const catalog = await fetchCatalog();
  if (!catalog.providers || !catalog.models) return [];
  return catalog.providers.filter(p => p.ai_sdk_package).map(p => ({
    id: p.id, name: p.name, sdkPackage: p.ai_sdk_package ?? "", apiKeyEnv: p.api_key_env ?? "", docsUrl: p.docs_url, baseUrl: p.base_url,
    models: catalog.models.filter(m => m.id.startsWith(`${p.id}/`)).map(m => ({
      id: m.id, provider: p.id, sdkModel: m.id, name: m.name, contextWindow: m.context_window ?? 4096, supportsTools: m.supports_tools ?? false,
      supportsStructuredOutput: m.supports_structured_output ?? false, supportsStreaming: m.supports_streaming ?? true,
      pricing: { input: m.pricing?.input ?? 0, output: m.pricing?.output ?? 0, cachedInput: m.pricing?.cached_input },
      openWeights: m.open_weights ?? false, released: m.released,
    })),
  }));
}

export async function getModels(): Promise<ModelInfo[]> {
  const providers = await getProviders();
  return providers.flatMap(p => p.models);
}
