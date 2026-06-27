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

// ===================== GitHub integration =====================

/// Whether GitHub OAuth is configured/connected, reported to the UI.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitHubStatus {
    /// True if the server has an OAuth client id+secret configured.
    pub configured: bool,
    /// True if the current browser session is authenticated.
    pub connected: bool,
    /// The authenticated GitHub login, when connected.
    #[serde(default)]
    pub login: Option<String>,
}

/// A repository the user can browse.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepoInfo {
    pub full_name: String,
    pub default_branch: String,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub description: Option<String>,
}

/// One entry in a repository directory listing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepoEntry {
    pub name: String,
    pub path: String,
    /// "file" or "dir".
    pub kind: String,
    #[serde(default)]
    pub sha: String,
}

/// A file opened from a repository (content + blob sha needed to commit back).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepoFile {
    pub repo: String,
    pub path: String,
    pub content: String,
    pub sha: String,
}

/// Request to commit (create/update) a file in a repo.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommitRequest {
    pub repo: String,
    pub path: String,
    pub content: String,
    pub message: String,
    /// Blob sha of the file being replaced; omit for a brand-new file.
    #[serde(default)]
    pub sha: Option<String>,
}

// ===================== Scaffolding =====================

/// A single generated file within a scaffold.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScaffoldFile {
    pub path: String,
    pub content: String,
}

/// A project template the user can scaffold.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScaffoldTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub language: String,
    pub files: Vec<ScaffoldFile>,
}

/// Request to scaffold a template into a brand-new GitHub repository.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScaffoldToRepoRequest {
    pub template: String,
    pub repo_name: String,
    #[serde(default)]
    pub private: bool,
}

fn f(path: &str, content: &str) -> ScaffoldFile {
    ScaffoldFile {
        path: path.to_string(),
        content: content.to_string(),
    }
}

/// The built-in project templates. Shared by the WASM UI (client-side
/// "load into editor") and the server (push to a new repo).
pub fn scaffold_templates() -> Vec<ScaffoldTemplate> {
    vec![
        ScaffoldTemplate {
            id: "rust-cli".into(),
            name: "Rust CLI".into(),
            description: "A minimal Rust binary crate.".into(),
            language: "rust".into(),
            files: vec![
                f(
                    "Cargo.toml",
                    "[package]\nname = \"my-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
                ),
                f(
                    "src/main.rs",
                    "fn main() {\n    println!(\"Hello from a T.C.K-scaffolded Rust app!\");\n}\n",
                ),
                f("README.md", "# my-app\n\nScaffolded by T.C.K.\n\n```bash\ncargo run\n```\n"),
                f(".gitignore", "/target\n"),
            ],
        },
        ScaffoldTemplate {
            id: "static-web".into(),
            name: "Static web page".into(),
            description: "HTML + CSS + JS starter.".into(),
            language: "web".into(),
            files: vec![
                f(
                    "index.html",
                    "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\" />\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n  <title>My Site</title>\n  <link rel=\"stylesheet\" href=\"style.css\" />\n</head>\n<body>\n  <h1>Hello</h1>\n  <p>Scaffolded by T.C.K.</p>\n  <script src=\"script.js\"></script>\n</body>\n</html>\n",
                ),
                f(
                    "style.css",
                    "body { font-family: system-ui, sans-serif; margin: 3rem auto; max-width: 40rem; }\nh1 { color: #0e639c; }\n",
                ),
                f("script.js", "console.log('Hello from T.C.K');\n"),
                f("README.md", "# My Site\n\nScaffolded by T.C.K. Open `index.html`.\n"),
            ],
        },
        ScaffoldTemplate {
            id: "node-script".into(),
            name: "Node.js script".into(),
            description: "A small Node package.".into(),
            language: "javascript".into(),
            files: vec![
                f(
                    "package.json",
                    "{\n  \"name\": \"my-app\",\n  \"version\": \"0.1.0\",\n  \"type\": \"module\",\n  \"scripts\": { \"start\": \"node index.js\" }\n}\n",
                ),
                f("index.js", "console.log('Hello from a T.C.K-scaffolded Node app!');\n"),
                f("README.md", "# my-app\n\nScaffolded by T.C.K.\n\n```bash\nnpm start\n```\n"),
                f(".gitignore", "node_modules/\n"),
            ],
        },
        ScaffoldTemplate {
            id: "python-script".into(),
            name: "Python script".into(),
            description: "A minimal Python project.".into(),
            language: "python".into(),
            files: vec![
                f("main.py", "def main():\n    print(\"Hello from a T.C.K-scaffolded Python app!\")\n\n\nif __name__ == \"__main__\":\n    main()\n"),
                f("requirements.txt", ""),
                f("README.md", "# my-app\n\nScaffolded by T.C.K.\n\n```bash\npython main.py\n```\n"),
                f(".gitignore", "__pycache__/\n*.pyc\n.venv/\n"),
            ],
        },
    ]
}
