// Shared types for the tck-ai service

/** A request to generate text via the Vercel AI SDK. */
export interface GenerateRequest {
  /** Provider key: "anthropic", "openai", "google", "groq", "mistral", "deepseek", "xai", "openai-compatible" */
  provider: string;
  /** Model ID (e.g. "claude-sonnet-5", "gpt-4o") */
  model: string;
  /** The user prompt */
  prompt: string;
  /** Optional system prompt */
  system?: string;
  /** API key (from user settings) */
  apiKey?: string;
  /** Base URL for openai-compatible providers */
  baseUrl?: string;
  /** Temperature (0-2) */
  temperature?: number;
  /** Max tokens to generate */
  maxTokens?: number;
}

/** Response from a generate call. */
export interface GenerateResponse {
  response: string;
  model: string;
  usage?: {
    promptTokens: number;
    completionTokens: number;
    totalTokens: number;
  };
}

/** A model listed in the catalog. */
export interface ModelInfo {
  id: string;
  /** Provider key matching the SDK */
  provider: string;
  /** AI SDK model string (e.g. "anthropic/claude-sonnet-5") */
  sdkModel: string;
  /** Human-readable name */
  name: string;
  /** Context window in tokens */
  contextWindow: number;
  /** Supports tool calling */
  supportsTools: boolean;
  /** Supports structured output */
  supportsStructuredOutput: boolean;
  /** Supports streaming */
  supportsStreaming: boolean;
  /** Pricing in USD per million tokens */
  pricing: {
    input: number;
    output: number;
    cachedInput?: number;
  };
  /** Whether weights are open */
  openWeights: boolean;
  /** Release date ISO string */
  released?: string;
}

/** A provider that can be used. */
export interface ProviderInfo {
  id: string;
  name: string;
  /** AI SDK package name */
  sdkPackage: string;
  /** Environment variable name for the API key */
  apiKeyEnv: string;
  /** Docs URL */
  docsUrl?: string;
  /** Default base URL for API calls */
  baseUrl?: string;
  /** Models available through this provider */
  models: ModelInfo[];
}

/** The full models.dev catalog shape we consume. */
export interface ModelsDevCatalog {
  models: ModelsDevModel[];
  providers: ModelsDevProvider[];
}

export interface ModelsDevModel {
  id: string;
  name: string;
  description?: string;
  context_window?: number;
  max_output?: number;
  supports_tools?: boolean;
  supports_structured_output?: boolean;
  supports_temperature?: boolean;
  supports_streaming?: boolean;
  open_weights?: boolean;
  released?: string;
  pricing?: {
    input?: number;
    output?: number;
    cached_input?: number;
  };
}

export interface ModelsDevProvider {
  id: string;
  name: string;
  ai_sdk_package?: string;
  api_key_env?: string;
  docs_url?: string;
  base_url?: string;
}