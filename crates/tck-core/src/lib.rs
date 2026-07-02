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
    KNOWN_AGENTS_LIST
        .iter()
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

// ===================== Token / usage tracking =====================

/// Tracked consumption for one AI tool or agent role across a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgentTokenUsage {
    pub agent_id: String,
    pub name: String,
    pub icon: String,
    /// Approximate token count (chars / 4) produced by this agent.
    pub tokens: usize,
    /// Raw character count produced.
    pub chars_produced: usize,
    /// How many AI calls this agent made.
    pub call_count: usize,
}

impl AgentTokenUsage {
    pub fn merge(&mut self, chars: usize) {
        self.chars_produced += chars;
        self.tokens = self.chars_produced / 4;
        self.call_count += 1;
    }
}

/// Helper: produce a share string like "42%".
pub fn usage_pct(chars: usize, total: usize) -> String {
    if total == 0 {
        "0%".into()
    } else {
        format!("{:.0}%", 100.0 * chars as f64 / total as f64)
    }
}

/// Return a fixed vibrant color for any agent id / role.
pub fn agent_color(id: &str) -> &'static str {
    match id {
        "claude" | "Claude Code" => "#d97706",
        "codex" | "Codex" => "#7c3aed",
        "opencode" | "OpenCode" => "#059669",
        "gemini" | "Gemini CLI" => "#2563eb",
        "aider" | "Aider" => "#dc2626",
        "planner" => "#06b6d4",
        "researcher" => "#3b82f6",
        "architect" => "#a855f7",
        "coder" => "#22c55e",
        "reviewer" => "#ef4444",
        "writer" => "#eab308",
        "synthesizer" => "#f97316",
        _ => "#94a3b8",
    }
}

/// Determine which "tool" bucket an agent belongs to based on role or id.
/// Used to aggregate token usage across orchestrator roles + known CLI tools.
pub fn bucket_agent(role_id: &str) -> (&'static str, &'static str) {
    match role_id {
        "planner" => ("planner", "🧭"),
        "researcher" => ("researcher", "🔎"),
        "architect" => ("architect", "📐"),
        "coder" => ("coder", "⌨️"),
        "reviewer" => ("reviewer", "🧐"),
        "writer" => ("writer", "✍️"),
        "synthesizer" => ("synthesizer", "🧵"),
        "claude" | "Claude Code" => ("claude", "⚡"),
        "codex" | "Codex" => ("codex", "🧩"),
        "opencode" | "OpenCode" => ("opencode", "📟"),
        "gemini" | "Gemini CLI" => ("gemini", "✦"),
        "aider" | "Aider" => ("aider", "🛠"),
        _ => ("tool", "🤖"),
    }
}

/// Static list used by `known_agents()`.
const KNOWN_AGENTS_LIST: &[(&str, &str, &str, &str)] = &[
    ("claude", "Claude Code", "claude", "⚡"),
    ("codex", "Codex", "codex", "🧩"),
    ("opencode", "OpenCode", "opencode", "📟"),
    ("gemini", "Gemini CLI", "gemini", "✦"),
    ("aider", "Aider", "aider", "🛠"),
];

// ===================== Multi-agent orchestrator =====================

/// A specialist role a worker agent can take in an orchestration run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentRole {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub system_prompt: String,
}

/// The worker roles the planner can assign subtasks to.
pub fn orchestrator_roles() -> Vec<AgentRole> {
    [
        (
            "researcher",
            "Researcher",
            "🔎",
            "You are the Researcher agent in a multi-agent team. Gather the relevant \
             facts, constraints, prior art, and pitfalls for your assigned subtask. \
             Be concrete and cite versions/APIs where relevant. Output a concise, \
             well-structured brief that other agents can build on.",
        ),
        (
            "architect",
            "Architect",
            "📐",
            "You are the Architect agent in a multi-agent team. Design the structure \
             for your assigned subtask: components, data flow, interfaces, and \
             trade-offs. Prefer the simplest design that satisfies the goal. Output \
             a clear design other agents can implement from.",
        ),
        (
            "coder",
            "Coder",
            "⌨️",
            "You are the Coder agent in a multi-agent team. Produce working, complete \
             code for your assigned subtask. Include all necessary files/snippets in \
             fenced code blocks with filenames, and note any assumptions.",
        ),
        (
            "reviewer",
            "Reviewer",
            "🧐",
            "You are the Reviewer agent in a multi-agent team. Critically examine the \
             plan and your assigned subtask for bugs, edge cases, security issues, \
             and gaps. Output a prioritized list of concrete findings and fixes.",
        ),
        (
            "writer",
            "Writer",
            "✍️",
            "You are the Writer agent in a multi-agent team. Produce clear prose — \
             documentation, explanations, or user-facing text — for your assigned \
             subtask. Be accurate and concise.",
        ),
    ]
    .into_iter()
    .map(|(id, name, icon, system_prompt)| AgentRole {
        id: id.to_string(),
        name: name.to_string(),
        icon: icon.to_string(),
        system_prompt: system_prompt.to_string(),
    })
    .collect()
}

/// Look up a role by id, falling back to a generic worker.
pub fn role_by_id(id: &str) -> AgentRole {
    orchestrator_roles()
        .into_iter()
        .find(|r| r.id == id)
        .unwrap_or(AgentRole {
            id: id.to_string(),
            name: id.to_string(),
            icon: "🤖".to_string(),
            system_prompt: "You are a capable agent in a multi-agent team. Complete \
                            your assigned subtask thoroughly and output your result."
                .to_string(),
        })
}

/// One subtask produced by the planner agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PlannedTask {
    pub role: String,
    pub title: String,
    pub instructions: String,
}

/// Maximum number of worker tasks a plan may contain.
pub const MAX_PLAN_TASKS: usize = 5;

/// System prompt for the planner agent.
pub fn planner_system() -> String {
    let roles: Vec<String> = orchestrator_roles()
        .iter()
        .map(|r| format!("\"{}\" ({})", r.id, r.name))
        .collect();
    format!(
        "You are the Planner agent of a multi-agent orchestrator. Decompose the \
         user's goal into 2 to {MAX_PLAN_TASKS} independent subtasks that can run in \
         parallel, each assigned to one specialist role from: {}. Respond with ONLY a \
         JSON array, no prose and no code fences, where each element is \
         {{\"role\": \"<role id>\", \"title\": \"<short title>\", \"instructions\": \
         \"<detailed instructions for that agent>\"}}.",
        roles.join(", ")
    )
}

/// User prompt for the planner agent.
pub fn planner_prompt(goal: &str) -> String {
    format!("Goal:\n{goal}\n\nProduce the JSON task plan now.")
}

/// Prompt handed to a worker agent for one subtask.
pub fn worker_prompt(goal: &str, task: &PlannedTask) -> String {
    format!(
        "Overall goal:\n{goal}\n\nYour subtask — {title}:\n{instructions}\n\n\
         Complete this subtask now and output your result.",
        title = task.title,
        instructions = task.instructions
    )
}

/// System prompt for the synthesizer agent.
pub fn synthesizer_system() -> String {
    "You are the Synthesizer agent of a multi-agent orchestrator. You receive the \
     user's goal and the outputs of several specialist agents. Merge them into one \
     coherent, deduplicated final answer that directly satisfies the goal. Resolve \
     conflicts between agents sensibly and note anything left open."
        .to_string()
}

/// User prompt for the synthesizer agent, given each task's output.
pub fn synthesizer_prompt(goal: &str, results: &[(PlannedTask, String)]) -> String {
    let mut out = format!("Goal:\n{goal}\n");
    for (task, output) in results {
        out.push_str(&format!(
            "\n--- Agent \"{}\" ({}) — {} ---\n{}\n",
            role_by_id(&task.role).name,
            task.role,
            task.title,
            output
        ));
    }
    out.push_str("\nProduce the final combined answer now.");
    out
}

/// Extract the planner's JSON task array from raw model output.
///
/// Models often wrap JSON in code fences or prose, so this scans for the first
/// balanced top-level `[...]` (string-aware) and tries to parse it. Returns an
/// empty vec if no valid plan is found; callers should fall back gracefully.
pub fn parse_plan(text: &str) -> Vec<PlannedTask> {
    let bytes = text.as_bytes();
    let mut start = None;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate() {
        if in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' if start.is_some() => in_str = true,
            b'[' => {
                if start.is_none() {
                    start = Some(i);
                }
                depth += 1;
            }
            b']' if start.is_some() => {
                depth -= 1;
                if depth == 0 {
                    let candidate = &text[start.unwrap()..=i];
                    if let Ok(tasks) = serde_json::from_str::<Vec<PlannedTask>>(candidate) {
                        let tasks: Vec<PlannedTask> = tasks
                            .into_iter()
                            .filter(|t| !t.instructions.trim().is_empty())
                            .take(MAX_PLAN_TASKS)
                            .collect();
                        if !tasks.is_empty() {
                            return tasks;
                        }
                    }
                    // Not a valid plan — keep scanning after this bracket.
                    start = None;
                }
            }
            _ => {}
        }
    }
    Vec::new()
}

/// The fallback single-task plan used when the planner output can't be parsed.
pub fn fallback_plan(goal: &str) -> Vec<PlannedTask> {
    vec![PlannedTask {
        role: "coder".to_string(),
        title: "Complete the goal".to_string(),
        instructions: goal.to_string(),
    }]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plan_bare_json_array() {
        let text = r#"[{"role":"coder","title":"Write it","instructions":"Do the thing."}]"#;
        let plan = parse_plan(text);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].role, "coder");
        assert_eq!(plan[0].title, "Write it");
    }

    #[test]
    fn parse_plan_in_code_fence_with_prose() {
        let text = "Here is the plan:\n```json\n[\n  {\"role\": \"researcher\", \"title\": \"R\", \"instructions\": \"research [stuff]\"},\n  {\"role\": \"coder\", \"title\": \"C\", \"instructions\": \"code it\"}\n]\n```\nGood luck!";
        let plan = parse_plan(text);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[1].role, "coder");
        assert_eq!(plan[0].instructions, "research [stuff]");
    }

    #[test]
    fn parse_plan_handles_brackets_inside_strings() {
        let text = r#"[{"role":"writer","title":"T ] tricky","instructions":"use arr[0] and \"quotes\""}]"#;
        let plan = parse_plan(text);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].title, "T ] tricky");
    }

    #[test]
    fn parse_plan_skips_earlier_non_plan_array_and_caps_tasks() {
        let mut tasks = String::from("The roles are [1, 2, 3]. Plan: [");
        for i in 0..8 {
            if i > 0 {
                tasks.push(',');
            }
            tasks.push_str(&format!(
                r#"{{"role":"coder","title":"t{i}","instructions":"do {i}"}}"#
            ));
        }
        tasks.push(']');
        let plan = parse_plan(&tasks);
        assert_eq!(plan.len(), MAX_PLAN_TASKS);
        assert_eq!(plan[0].title, "t0");
    }

    #[test]
    fn parse_plan_rejects_garbage_and_empty_instructions() {
        assert!(parse_plan("no json here").is_empty());
        assert!(parse_plan("[]").is_empty());
        assert!(parse_plan(r#"[{"role":"coder","title":"x","instructions":"  "}]"#).is_empty());
    }

    #[test]
    fn role_lookup_falls_back_to_generic() {
        assert_eq!(role_by_id("coder").name, "Coder");
        let unknown = role_by_id("weird-role");
        assert_eq!(unknown.id, "weird-role");
        assert!(!unknown.system_prompt.is_empty());
    }

    #[test]
    fn prompts_mention_goal_and_tasks() {
        let task = PlannedTask {
            role: "coder".into(),
            title: "Build".into(),
            instructions: "Build the app".into(),
        };
        assert!(planner_system().contains("JSON array"));
        assert!(planner_prompt("make a game").contains("make a game"));
        assert!(worker_prompt("make a game", &task).contains("Build the app"));
        let synth = synthesizer_prompt("make a game", &[(task, "done".into())]);
        assert!(synth.contains("make a game"));
        assert!(synth.contains("done"));
    }
}
