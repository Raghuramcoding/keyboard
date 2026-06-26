//! Shared types for T.C.K (TalentCloud Keyboard).
//!
//! These structs are serialized over the wire between the WASM UI and the
//! native server, and are also used directly inside the UI's local state.

use serde::{Deserialize, Serialize};

/// An OpenAI-compatible AI provider the user has configured.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub models: Vec<String>,
}

/// A user-defined button that launches a command in the terminal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CustomCommand {
    pub id: String,
    pub label: String,
    pub command: String,
    #[serde(default)]
    pub icon: String,
}

/// Persisted application settings (stored in the browser's localStorage).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
    #[serde(default)]
    pub commands: Vec<CustomCommand>,
    #[serde(default)]
    pub claude_api_key: String,
    #[serde(default)]
    pub ollama_host: String,
    #[serde(default)]
    pub background: String,
}

/// Request sent to the server's `/api/generate` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    /// "ollama", "claude", or a provider id.
    pub provider: String,
    pub model: String,
    pub prompt: String,
    #[serde(default)]
    pub system: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// Response from `/api/generate`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerateResponse {
    #[serde(default)]
    pub response: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// A known AI coding agent we can auto-detect and launch in the terminal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Agent {
    pub id: String,
    pub label: String,
    pub command: String,
    pub icon: String,
}

/// The agents T.C.K knows how to launch.
pub fn known_agents() -> Vec<Agent> {
    [
        ("claude", "Claude Code", "claude", "⚡"),
        ("codex", "Codex", "codex", "🧩"),
        ("opencode", "OpenCode", "opencode", "📟"),
        ("gemini", "Gemini CLI", "gemini", "✦"),
        ("aider", "Aider", "aider", "🛠"),
    ]
    .into_iter()
    .map(|(id, label, command, icon)| Agent {
        id: id.to_string(),
        label: label.to_string(),
        command: command.to_string(),
        icon: icon.to_string(),
    })
    .collect()
}

/// Built-in OpenAI-compatible provider presets (name, base URL).
pub fn provider_presets() -> Vec<(&'static str, &'static str)> {
    vec![
        ("OpenAI", "https://api.openai.com/v1"),
        ("OpenRouter", "https://openrouter.ai/api/v1"),
        ("Groq", "https://api.groq.com/openai/v1"),
        ("Together", "https://api.together.xyz/v1"),
        ("DeepSeek", "https://api.deepseek.com/v1"),
        ("Mistral", "https://api.mistral.ai/v1"),
        ("OpenCode Zen", "https://opencode.ai/zen/v1"),
        ("LM Studio (local)", "http://localhost:1234/v1"),
        ("llama.cpp (local)", "http://localhost:8080/v1"),
    ]
}

/// Built-in Claude models offered when an API key is present.
pub fn claude_models() -> Vec<&'static str> {
    vec!["claude-opus-4-8", "claude-sonnet-4-6", "claude-haiku-4-5"]
}
