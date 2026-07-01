//! T.C.K — TalentCloud Keyboard
//! A pure-Rust (Dioxus/WASM) AI coding environment that runs in the browser.

mod landing;

use dioxus::prelude::*;
use futures_util::{SinkExt, StreamExt};
use gloo_net::http::Request;
use gloo_net::websocket::{futures::WebSocket, Message as WsMessage};
use gloo_storage::{LocalStorage, Storage};
use landing::LandingPage;
use serde_json::{json, Value};
use tck_core::{
    claude_models, known_agents, provider_presets, scaffold_templates, CommitRequest,
    CustomCommand, GenerateRequest, GenerateResponse, GitHubStatus, ProviderConfig, RepoEntry,
    RepoFile, RepoInfo, ScaffoldToRepoRequest, Settings,
};
use wasm_bindgen::JsCast;

const SETTINGS_KEY: &str = "tck.settings";

fn main() {
    dioxus::launch(App);
}

// ----- chat -----
#[derive(Clone, PartialEq)]
enum Role {
    User,
    Ai,
    Err,
}

#[derive(Clone, PartialEq)]
struct ChatMsg {
    role: Role,
    text: String,
}

// ----- mobile layout -----
// On phones (narrow screens) only one panel is shown at a time, chosen by a
// bottom tab bar. On wide screens all panels are visible and the tab bar hides.
#[derive(Clone, Copy, PartialEq)]
enum MobileTab {
    Agents,
    Editor,
    Terminal,
    Ai,
}

impl MobileTab {
    fn class(self) -> &'static str {
        match self {
            MobileTab::Agents => "view-agents",
            MobileTab::Editor => "view-editor",
            MobileTab::Terminal => "view-terminal",
            MobileTab::Ai => "view-ai",
        }
    }
}

// ----- shared state -----
#[derive(Clone, Copy)]
struct AppState {
    settings: Signal<Settings>,
    code: Signal<String>,
    filename: Signal<String>,
    provider: Signal<String>,
    model: Signal<String>,
    chat: Signal<Vec<ChatMsg>>,
    busy: Signal<bool>,
    show_settings: Signal<bool>,
    term_lines: Signal<String>,
    term_connected: Signal<bool>,
    term: Coroutine<String>,
    mobile_tab: Signal<MobileTab>,
    /// True once a native T.C.K server is detected (via /api/health). When false
    /// (e.g. served from GitHub Pages) AI calls go directly to the provider.
    server: Signal<bool>,
    // ----- GitHub / source control -----
    gh: Signal<GitHubStatus>,
    repos: Signal<Vec<RepoInfo>>,
    /// Current repo being browsed and the directory path within it.
    browse_repo: Signal<Option<String>>,
    browse_path: Signal<String>,
    entries: Signal<Vec<RepoEntry>>,
    /// (repo, path, sha) of the file currently open from a repo, enabling Commit.
    open_file: Signal<Option<(String, String, String)>>,
    git_msg: Signal<String>,
}

fn load_settings() -> Settings {
    LocalStorage::get(SETTINGS_KEY).unwrap_or_default()
}

fn save_settings(s: &Settings) {
    let _ = LocalStorage::set(SETTINGS_KEY, s);
}

/// Strip ANSI/VT escape sequences and stray control bytes so raw PTY output is
/// legible in a plain `<pre>` (we have no xterm.js terminal emulator here).
fn sanitize_terminal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => match chars.next() {
                // CSI: ESC [ ... <final 0x40..=0x7E>
                Some('[') => {
                    for n in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&n) {
                            break;
                        }
                    }
                }
                // OSC: ESC ] ... (BEL | ESC \)
                Some(']') => {
                    while let Some(n) = chars.next() {
                        if n == '\u{07}' {
                            break;
                        }
                        if n == '\u{1b}' {
                            chars.next();
                            break;
                        }
                    }
                }
                // Two-char escapes like ESC ( B — drop the following byte.
                Some('(') | Some(')') | Some('#') => {
                    chars.next();
                }
                _ => {}
            },
            // Keep newlines, carriage returns, and tabs; drop other controls.
            '\n' | '\r' | '\t' => out.push(c),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {}
            c => out.push(c),
        }
    }
    out
}

fn terminal_offline_msg() -> String {
    "[T.C.K terminal offline]\r\nThe terminal needs the native T.C.K server, which can't run on a\r\nstatic host like GitHub Pages. To use it, run the server locally:\r\n\r\n    cargo run -p tck-server\r\n\r\nThen reload this page from http://127.0.0.1:3000\r\n".to_string()
}

fn ws_url() -> String {
    let loc = web_sys::window().unwrap().location();
    let proto = if loc.protocol().unwrap_or_default() == "https:" {
        "wss"
    } else {
        "ws"
    };
    let host = loc.host().unwrap_or_else(|_| "127.0.0.1:8080".into());
    format!("{proto}://{host}/ws/pty")
}

#[component]
fn App() -> Element {
    let mut show_landing = use_signal(check_show_landing);

    // A link from the landing page into the app only changes the URL hash
    // (same document, no reload), so re-check on every `hashchange` rather
    // than just once at mount.
    use_effect(move || {
        if let Some(window) = web_sys::window() {
            let win = window.clone();
            let closure = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
                show_landing.set(check_show_landing_in(&win));
            });
            let _ = window
                .add_event_listener_with_callback("hashchange", closure.as_ref().unchecked_ref());
            closure.forget();
        }
    });

    if *show_landing.read() {
        return rsx! {
            LandingPage {}
        };
    }

    rsx! { AppMain {} }
}

fn check_show_landing() -> bool {
    web_sys::window()
        .map(|w| check_show_landing_in(&w))
        .unwrap_or(true)
}

fn check_show_landing_in(window: &web_sys::Window) -> bool {
    match window.location().hash() {
        Ok(hash) => hash.is_empty() || hash == "#",
        Err(_) => true,
    }
}

#[component]
fn AppMain() -> Element {
    let settings = use_signal(load_settings);
    let code = use_signal(|| {
        "// Welcome to T.C.K — TalentCloud Keyboard\n// A pure-Rust AI coding environment running in your browser.\n\nfn main() {\n    println!(\"Hello from T.C.K!\");\n}\n"
            .to_string()
    });
    let filename = use_signal(|| "scratch.rs".to_string());
    let provider = use_signal(|| "ollama".to_string());
    let model = use_signal(String::new);
    let chat = use_signal(Vec::<ChatMsg>::new);
    let busy = use_signal(|| false);
    let show_settings = use_signal(|| false);
    let mobile_tab = use_signal(|| MobileTab::Editor);
    let mut server = use_signal(|| false);
    let gh = use_signal(GitHubStatus::default);
    let repos = use_signal(Vec::<RepoInfo>::new);
    let browse_repo = use_signal(|| Option::<String>::None);
    let browse_path = use_signal(String::new);
    let entries = use_signal(Vec::<RepoEntry>::new);
    let open_file = use_signal(|| Option::<(String, String, String)>::None);
    let git_msg = use_signal(String::new);
    let mut term_lines = use_signal(String::new);
    let mut term_connected = use_signal(|| false);

    // Detect a native T.C.K server. On a static host (GitHub Pages) this 404s and
    // we stay in "serverless" mode: AI calls go straight to the provider.
    let mut gh_w = gh;
    use_future(move || async move {
        if let Ok(resp) = Request::get("/api/health").send().await {
            if resp.ok() {
                if let Ok(t) = resp.text().await {
                    if t.trim() == "ok" {
                        server.set(true);
                    }
                }
            }
        }
        // Also learn GitHub status (configured / already connected).
        if let Some(s) = fetch_gh_status().await {
            gh_w.set(s);
        }
    });

    // Terminal: connect to the server's PTY bridge over a WebSocket. Anything
    // sent to this coroutine is written to the shell; output streams back in.
    let term = use_coroutine(move |mut rx: UnboundedReceiver<String>| async move {
        match WebSocket::open(&ws_url()) {
            Ok(ws) => {
                let (mut write, mut read) = ws.split();
                // Mark connected on the first received byte; if the stream ends
                // without any data (e.g. a static host with no PTY bridge), show
                // the offline notice instead of leaving the pane blank.
                spawn(async move {
                    let mut received = false;
                    while let Some(item) = read.next().await {
                        let msg = match item {
                            Ok(m) => m,
                            Err(_) => break,
                        };
                        if !received {
                            received = true;
                            term_connected.set(true);
                        }
                        let text = match msg {
                            WsMessage::Text(t) => t,
                            WsMessage::Bytes(b) => String::from_utf8_lossy(&b).to_string(),
                        };
                        let mut cur = term_lines.peek().clone();
                        cur.push_str(&sanitize_terminal(&text));
                        if cur.len() > 120_000 {
                            cur = cur.split_off(cur.len() - 100_000);
                        }
                        term_lines.set(cur);
                    }
                    term_connected.set(false);
                    if !received {
                        term_lines.set(terminal_offline_msg());
                    }
                });
                while let Some(input) = rx.next().await {
                    if write.send(WsMessage::Text(input)).await.is_err() {
                        break;
                    }
                }
            }
            Err(_) => {
                term_lines.set(terminal_offline_msg());
                term_connected.set(false);
            }
        }
    });

    let state = use_context_provider(|| AppState {
        settings,
        code,
        filename,
        provider,
        model,
        chat,
        busy,
        show_settings,
        term_lines,
        term_connected,
        term,
        mobile_tab,
        server,
        gh,
        repos,
        browse_repo,
        browse_path,
        entries,
        open_file,
        git_msg,
    });

    let view_class = state.mobile_tab.read().class();

    rsx! {
        div { class: "app",
            TitleBar {}
            div { class: "body {view_class}",
                Sidebar {}
                div { class: "center",
                    Editor {}
                    TerminalPanel {}
                }
                ChatPanel {}
            }
            MobileTabBar {}
            if *state.show_settings.read() {
                SettingsModal {}
            }
        }
    }
}

#[component]
fn MobileTabBar() -> Element {
    let mut state = use_context::<AppState>();
    let active = *state.mobile_tab.read();
    let tabs = [
        (MobileTab::Agents, "🧰", "Tools"),
        (MobileTab::Editor, "📝", "Editor"),
        (MobileTab::Terminal, "💻", "Terminal"),
        (MobileTab::Ai, "🤖", "AI"),
    ];
    rsx! {
        div { class: "mobile-tabbar",
            for (tab , icon , label) in tabs.iter() {
                button {
                    key: "{label}",
                    class: if active == *tab { "mtab active" } else { "mtab" },
                    onclick: {
                        let t = *tab;
                        move |_| state.mobile_tab.set(t)
                    },
                    span { class: "mtab-icon", "{icon}" }
                    span { class: "mtab-label", "{label}" }
                }
            }
        }
    }
}

#[component]
fn TitleBar() -> Element {
    let mut state = use_context::<AppState>();
    let settings = state.settings.read().clone();

    // Provider options: ollama, claude (if key), then each configured provider.
    let mut options: Vec<(String, String)> = vec![("ollama".into(), "Ollama".into())];
    if !settings.claude_api_key.is_empty() {
        options.push(("claude".into(), "Claude".into()));
    }
    for p in &settings.providers {
        options.push((p.id.clone(), p.name.clone()));
    }

    let provider_now = state.provider.read().clone();
    let suggestions = model_suggestions(&provider_now, &settings);

    rsx! {
        div { class: "titlebar",
            div { class: "brand",
                span { "⌨️ T.C.K" }
                span { class: "sub", "TalentCloud Keyboard" }
            }
            div { class: "spacer" }
            select {
                value: "{provider_now}",
                onchange: move |e| {
                    state.provider.set(e.value());
                    state.model.set(String::new());
                },
                for (id , label) in options.iter() {
                    option { key: "{id}", value: "{id}", "{label}" }
                }
            }
            input {
                list: "model-suggestions",
                placeholder: "model…",
                value: "{state.model}",
                oninput: move |e| state.model.set(e.value()),
            }
            datalist { id: "model-suggestions",
                for m in suggestions.iter() {
                    option { key: "{m}", value: "{m}" }
                }
            }
            button {
                class: "primary",
                onclick: move |_| state.show_settings.set(true),
                "⚙ Settings"
            }
        }
    }
}

fn model_suggestions(provider: &str, settings: &Settings) -> Vec<String> {
    match provider {
        "ollama" => ["llama3", "qwen2.5-coder", "deepseek-r1", "phi4"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        "claude" => claude_models().iter().map(|s| s.to_string()).collect(),
        id => settings
            .providers
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.models.clone())
            .unwrap_or_default(),
    }
}

#[component]
fn Sidebar() -> Element {
    let state = use_context::<AppState>();
    let settings = state.settings.read().clone();
    let agents = known_agents();

    rsx! {
        div { class: "sidebar",
            div {
                div { class: "section-title", "AI Agents" }
                div { class: "btn-col",
                    for a in agents.iter() {
                        button {
                            key: "{a.id}",
                            onclick: {
                                let cmd = a.command.clone();
                                move |_| state.term.send(format!("{cmd}\r"))
                            },
                            "{a.icon}  {a.label}"
                        }
                    }
                }
            }
            div {
                div { class: "section-title", "Custom Commands" }
                div { class: "btn-col",
                    if settings.commands.is_empty() {
                        div { class: "muted", "Add commands in Settings." }
                    }
                    for c in settings.commands.iter() {
                        button {
                            key: "{c.id}",
                            onclick: {
                                let cmd = c.command.clone();
                                move |_| state.term.send(format!("{cmd}\r"))
                            },
                            "{c.icon}  {c.label}"
                        }
                    }
                }
            }
            SourceControl {}
            ScaffoldPanel {}
        }
    }
}

#[component]
fn Editor() -> Element {
    let mut state = use_context::<AppState>();
    let open_file = state.open_file.read().clone();
    rsx! {
        div { class: "tabbar",
            div { class: "tab active", "{state.filename}" }
            if open_file.is_some() {
                button {
                    class: "commit-btn",
                    title: "Commit this file to GitHub",
                    onclick: move |_| commit_current(state),
                    "⬆ Commit"
                }
            }
        }
        div { class: "editor-wrap",
            textarea {
                class: "editor",
                spellcheck: false,
                value: "{state.code}",
                oninput: move |e| state.code.set(e.value()),
            }
        }
    }
}

#[component]
fn SourceControl() -> Element {
    let state = use_context::<AppState>();
    let gh = state.gh.read().clone();
    let repos = state.repos.read().clone();
    let entries = state.entries.read().clone();
    let browse_repo = state.browse_repo.read().clone();
    let browse_path = state.browse_path.read().clone();
    let msg = state.git_msg.read().clone();

    rsx! {
        div {
            div { class: "section-title", "Source Control" }
            if !gh.configured {
                div { class: "muted",
                    "GitHub isn't configured on the server. Set TCK_GITHUB_CLIENT_ID and TCK_GITHUB_CLIENT_SECRET, then restart."
                }
            } else if !gh.connected {
                a { class: "gh-btn", href: "/auth/github/login", "🔗 Connect GitHub" }
            } else {
                div { class: "gh-user",
                    span { "✓ {gh.login.clone().unwrap_or_default()}" }
                    button { class: "linkish", onclick: move |_| gh_logout(state), "Disconnect" }
                }
                div { class: "btn-col",
                    button { onclick: move |_| load_repos(state), "↻ Load my repos" }
                }
                if let Some(repo) = browse_repo.clone() {
                    div { class: "gh-crumb", "{repo}/{browse_path}" }
                    div { class: "btn-col",
                        button { class: "linkish", onclick: move |_| open_repo(state, repo.clone()), "⬅ repo root" }
                        if !browse_path.is_empty() {
                            button { class: "linkish", onclick: move |_| go_up(state), ".. up" }
                        }
                    }
                    div { class: "gh-list",
                        for e in entries.iter() {
                            button { key: "{e.path}", class: "gh-entry",
                                onclick: { let e2 = e.clone(); move |_| open_entry(state, e2.clone()) },
                                if e.kind == "dir" { "📁 {e.name}" } else { "📄 {e.name}" }
                            }
                        }
                    }
                } else {
                    div { class: "gh-list",
                        for r in repos.iter() {
                            button { key: "{r.full_name}", class: "gh-entry",
                                onclick: { let name = r.full_name.clone(); move |_| open_repo(state, name.clone()) },
                                "📦 {r.full_name}"
                            }
                        }
                    }
                }
            }
            if !msg.is_empty() {
                div { class: "gh-msg", "{msg}" }
            }
        }
    }
}

#[component]
fn ScaffoldPanel() -> Element {
    let state = use_context::<AppState>();
    let templates = scaffold_templates();
    let connected = state.gh.read().connected;
    let mut sel = use_signal(|| templates.first().map(|t| t.id.clone()).unwrap_or_default());
    let mut repo_name = use_signal(String::new);

    rsx! {
        div {
            div { class: "section-title", "Scaffold" }
            select {
                style: "width:100%;margin-bottom:6px;",
                value: "{sel}",
                onchange: move |e| sel.set(e.value()),
                for t in templates.iter() {
                    option { key: "{t.id}", value: "{t.id}", "{t.name}" }
                }
            }
            div { class: "btn-col",
                button { onclick: move |_| load_template_into_editor(state, sel.peek().clone()), "📄 Load into editor" }
                if connected {
                    input {
                        placeholder: "new-repo-name",
                        value: "{repo_name}",
                        oninput: move |e| repo_name.set(e.value()),
                    }
                    button {
                        onclick: move |_| create_scaffold_repo(state, sel.peek().clone(), repo_name.peek().clone()),
                        "🚀 Create GitHub repo"
                    }
                }
            }
        }
    }
}

// ----- source-control / scaffold actions -----

fn load_repos(state: AppState) {
    let mut repos = state.repos;
    let mut git_msg = state.git_msg;
    let mut browse_repo = state.browse_repo;
    git_msg.set("Loading repos…".to_string());
    spawn(async move {
        match fetch_repos().await {
            Ok(list) => {
                repos.set(list);
                browse_repo.set(None);
                git_msg.set(String::new());
            }
            Err(e) => git_msg.set(e),
        }
    });
}

fn open_repo(state: AppState, repo: String) {
    let mut browse_repo = state.browse_repo;
    let mut browse_path = state.browse_path;
    browse_repo.set(Some(repo.clone()));
    browse_path.set(String::new());
    load_entries(state, repo, String::new());
}

fn load_entries(state: AppState, repo: String, path: String) {
    let mut entries = state.entries;
    let mut git_msg = state.git_msg;
    git_msg.set("Loading…".to_string());
    spawn(async move {
        match fetch_contents(&repo, &path).await {
            Ok(c) => {
                entries.set(c.entries);
                git_msg.set(String::new());
            }
            Err(e) => git_msg.set(e),
        }
    });
}

fn open_entry(state: AppState, e: RepoEntry) {
    let repo = state.browse_repo.peek().clone().unwrap_or_default();
    if e.kind == "dir" {
        let mut browse_path = state.browse_path;
        browse_path.set(e.path.clone());
        load_entries(state, repo, e.path);
    } else {
        let mut code = state.code;
        let mut filename = state.filename;
        let mut open_file = state.open_file;
        let mut git_msg = state.git_msg;
        git_msg.set(format!("Opening {}…", e.name));
        spawn(async move {
            match fetch_contents(&repo, &e.path).await {
                Ok(c) => {
                    if let Some(file) = c.file {
                        code.set(file.content);
                        filename.set(e.name.clone());
                        open_file.set(Some((file.repo, file.path, file.sha)));
                        git_msg.set(String::new());
                    } else {
                        git_msg.set("That entry isn't a file.".to_string());
                    }
                }
                Err(err) => git_msg.set(err),
            }
        });
    }
}

fn go_up(state: AppState) {
    let repo = state.browse_repo.peek().clone().unwrap_or_default();
    let cur = state.browse_path.peek().clone();
    let parent = match cur.rfind('/') {
        Some(i) => cur[..i].to_string(),
        None => String::new(),
    };
    let mut browse_path = state.browse_path;
    browse_path.set(parent.clone());
    load_entries(state, repo, parent);
}

fn gh_logout(state: AppState) {
    let configured = state.gh.peek().configured;
    let mut gh = state.gh;
    let mut repos = state.repos;
    let mut browse_repo = state.browse_repo;
    let mut entries = state.entries;
    let mut open_file = state.open_file;
    let mut git_msg = state.git_msg;
    spawn(async move {
        let _ = Request::post("/api/github/logout").send().await;
        gh.set(GitHubStatus {
            configured,
            connected: false,
            login: None,
        });
        repos.set(vec![]);
        browse_repo.set(None);
        entries.set(vec![]);
        open_file.set(None);
        git_msg.set("Disconnected.".to_string());
    });
}

fn commit_current(state: AppState) {
    let Some((repo, path, sha)) = state.open_file.peek().clone() else {
        return;
    };
    let message = web_sys::window()
        .and_then(|w| w.prompt_with_message("Commit message:").ok().flatten())
        .unwrap_or_default();
    if message.trim().is_empty() {
        return;
    }
    let content = state.code.peek().clone();
    let mut git_msg = state.git_msg;
    let mut open_file = state.open_file;
    git_msg.set("Committing…".to_string());
    let req = CommitRequest {
        repo,
        path,
        content,
        message,
        sha: Some(sha),
    };
    spawn(async move {
        match commit_file(&req).await {
            Ok(new_sha) => {
                open_file.set(Some((req.repo, req.path, new_sha)));
                git_msg.set("✓ Committed.".to_string());
            }
            Err(e) => git_msg.set(format!("Commit failed: {e}")),
        }
    });
}

fn load_template_into_editor(state: AppState, id: String) {
    let Some(t) = scaffold_templates().into_iter().find(|t| t.id == id) else {
        return;
    };
    let Some(first) = t.files.first() else { return };
    let mut code = state.code;
    let mut filename = state.filename;
    let mut open_file = state.open_file;
    let mut git_msg = state.git_msg;
    code.set(first.content.clone());
    filename.set(first.path.clone());
    open_file.set(None);
    let names: Vec<String> = t.files.iter().map(|f| f.path.clone()).collect();
    git_msg.set(format!(
        "Loaded {} — {} files: {}",
        t.name,
        t.files.len(),
        names.join(", ")
    ));
}

fn create_scaffold_repo(state: AppState, template: String, repo_name: String) {
    if repo_name.trim().is_empty() {
        let mut git_msg = state.git_msg;
        git_msg.set("Enter a repo name first.".to_string());
        return;
    }
    let mut git_msg = state.git_msg;
    git_msg.set("Creating repo…".to_string());
    let req = ScaffoldToRepoRequest {
        template,
        repo_name,
        private: false,
    };
    spawn(async move {
        match scaffold_repo(&req).await {
            Ok(url) => git_msg.set(format!("✓ Created: {url}")),
            Err(e) => git_msg.set(format!("Failed: {e}")),
        }
    });
}

// ----- GitHub fetch helpers -----

async fn fetch_gh_status() -> Option<GitHubStatus> {
    let resp = Request::get("/api/github/status").send().await.ok()?;
    if !resp.ok() {
        return None;
    }
    resp.json::<GitHubStatus>().await.ok()
}

async fn fetch_repos() -> Result<Vec<RepoInfo>, String> {
    let resp = Request::get("/api/github/repos")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(api_err(resp).await);
    }
    resp.json::<Vec<RepoInfo>>()
        .await
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
struct ContentsResp {
    #[allow(dead_code)]
    kind: String,
    #[serde(default)]
    entries: Vec<RepoEntry>,
    #[serde(default)]
    file: Option<RepoFile>,
}

async fn fetch_contents(repo: &str, path: &str) -> Result<ContentsResp, String> {
    let url = format!(
        "/api/github/contents?repo={}&path={}",
        urlenc(repo),
        urlenc(path)
    );
    let resp = Request::get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(api_err(resp).await);
    }
    resp.json::<ContentsResp>().await.map_err(|e| e.to_string())
}

async fn commit_file(req: &CommitRequest) -> Result<String, String> {
    let resp = Request::put("/api/github/commit")
        .json(req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(api_err(resp).await);
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(v["sha"].as_str().unwrap_or_default().to_string())
}

async fn scaffold_repo(req: &ScaffoldToRepoRequest) -> Result<String, String> {
    let resp = Request::post("/api/scaffold/to-repo")
        .json(req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(api_err(resp).await);
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(v["html_url"].as_str().unwrap_or_default().to_string())
}

async fn api_err(resp: gloo_net::http::Response) -> String {
    match resp.json::<Value>().await {
        Ok(v) => v
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("request failed")
            .to_string(),
        Err(_) => "request failed".to_string(),
    }
}

fn urlenc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                o.push(b as char)
            }
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

#[component]
fn TerminalPanel() -> Element {
    let state = use_context::<AppState>();
    let connected = *state.term_connected.read();
    let mut input = use_signal(String::new);

    rsx! {
        div { class: "terminal",
            div { class: "term-head",
                span { class: if connected { "term-status on" } else { "term-status" } }
                span { if connected { "Terminal — connected" } else { "Terminal — offline" } }
            }
            pre { class: "term-out", "{state.term_lines}" }
            div { class: "term-in",
                input {
                    placeholder: "type a command and press Enter…",
                    value: "{input}",
                    oninput: move |e| input.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            let line = input.peek().clone();
                            state.term.send(format!("{line}\r"));
                            input.set(String::new());
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn ChatPanel() -> Element {
    let mut state = use_context::<AppState>();
    let mut prompt = use_signal(String::new);
    let busy = *state.busy.read();
    let messages = state.chat.read().clone();

    let send = move |_| {
        let text = prompt.peek().clone();
        if text.trim().is_empty() || *state.busy.peek() {
            return;
        }
        prompt.set(String::new());
        state.chat.write().push(ChatMsg {
            role: Role::User,
            text: text.clone(),
        });
        state.busy.set(true);
        let req = build_request(&state, text);
        let has_server = *state.server.peek();
        spawn(async move {
            let resp = call_generate(req, has_server).await;
            let msg = match (resp.response, resp.error) {
                (Some(r), _) => ChatMsg {
                    role: Role::Ai,
                    text: r,
                },
                (_, Some(e)) => ChatMsg {
                    role: Role::Err,
                    text: e,
                },
                _ => ChatMsg {
                    role: Role::Err,
                    text: "Empty response from provider.".into(),
                },
            };
            state.chat.write().push(msg);
            state.busy.set(false);
        });
    };

    rsx! {
        div { class: "chat",
            div { class: "chat-head", "🤖 AI Chat" }
            div { class: "chat-log",
                if messages.is_empty() {
                    div { class: "muted", "Ask the model anything. Responses route through the T.C.K server." }
                }
                for (i , m) in messages.iter().enumerate() {
                    div {
                        key: "{i}",
                        class: match m.role {
                            Role::User => "msg user",
                            Role::Ai => "msg ai",
                            Role::Err => "msg err",
                        },
                        div { class: "who",
                            match m.role { Role::User => "you", Role::Ai => "assistant", Role::Err => "error" }
                        }
                        "{m.text}"
                    }
                }
                if busy {
                    div { class: "msg ai", div { class: "who", "assistant" } "…thinking" }
                }
            }
            div { class: "chat-in",
                textarea {
                    placeholder: "Message the model…",
                    value: "{prompt}",
                    oninput: move |e| prompt.set(e.value()),
                }
                button { class: "primary", disabled: busy, onclick: send, "Send" }
            }
        }
    }
}

fn build_request(state: &AppState, prompt: String) -> GenerateRequest {
    let settings = state.settings.peek().clone();
    let provider = state.provider.peek().clone();
    let model = state.model.peek().clone();
    let mut req = GenerateRequest {
        provider: provider.clone(),
        model,
        prompt,
        system: Some("You are T.C.K, a helpful AI coding assistant.".into()),
        base_url: None,
        api_key: None,
        temperature: None,
        max_tokens: None,
    };
    match provider.as_str() {
        "ollama" => {
            if !settings.ollama_host.is_empty() {
                req.base_url = Some(settings.ollama_host);
            }
        }
        "claude" => {
            req.api_key = Some(settings.claude_api_key);
        }
        id => {
            if let Some(p) = settings.providers.iter().find(|p| p.id == id) {
                req.base_url = Some(p.base_url.clone());
                req.api_key = Some(p.api_key.clone());
            }
        }
    }
    req
}

async fn call_generate(req: GenerateRequest, has_server: bool) -> GenerateResponse {
    // With a native server present, proxy through it (no CORS, keys never touch
    // the page origin). On a static host (GitHub Pages) call the provider direct.
    if !has_server {
        return generate_direct(req).await;
    }
    let built = match Request::post("/api/generate").json(&req) {
        Ok(b) => b,
        Err(e) => {
            return GenerateResponse {
                error: Some(format!("Request build failed: {e}")),
                ..Default::default()
            }
        }
    };
    match built.send().await {
        Ok(resp) => resp
            .json::<GenerateResponse>()
            .await
            .unwrap_or_else(|e| GenerateResponse {
                error: Some(format!("Bad response from server: {e}")),
                ..Default::default()
            }),
        Err(e) => GenerateResponse {
            error: Some(format!(
                "Cannot reach the T.C.K server: {e}. Start it with `cargo run -p tck-server`."
            )),
            ..Default::default()
        },
    }
}

/// Serverless mode: call the AI provider straight from the browser. Works for
/// CORS-friendly providers (Anthropic with the browser-access header, OpenRouter,
/// a local Ollama with `OLLAMA_ORIGINS=*`, etc.).
async fn generate_direct(req: GenerateRequest) -> GenerateResponse {
    let result = match req.provider.as_str() {
        "ollama" => direct_ollama(&req).await,
        "claude" => direct_claude(&req).await,
        _ => direct_openai(&req).await,
    };
    match result {
        Ok(text) => GenerateResponse {
            response: Some(text),
            model: Some(req.model),
            ..Default::default()
        },
        Err(e) => GenerateResponse {
            error: Some(e),
            ..Default::default()
        },
    }
}

async fn direct_ollama(req: &GenerateRequest) -> Result<String, String> {
    let host = req
        .base_url
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://localhost:11434".to_string());
    let url = format!("{}/api/generate", host.trim_end_matches('/'));
    let body = json!({
        "model": req.model, "prompt": req.prompt,
        "system": req.system.clone().unwrap_or_default(), "stream": false,
    });
    let resp = Request::post(&url)
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| format!("Cannot reach Ollama: {e}"))?;
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Bad Ollama response: {e}"))?;
    if let Some(err) = v.get("error") {
        return Err(format!(
            "Ollama error: {}",
            err.as_str().unwrap_or("unknown")
        ));
    }
    Ok(v["response"].as_str().unwrap_or_default().to_string())
}

async fn direct_claude(req: &GenerateRequest) -> Result<String, String> {
    let key = req.api_key.clone().unwrap_or_default();
    if key.is_empty() {
        return Err("No Claude API key configured (set it in Settings).".to_string());
    }
    let body = json!({
        "model": req.model, "max_tokens": req.max_tokens.unwrap_or(2048),
        "system": req.system.clone().unwrap_or_default(),
        "messages": [{ "role": "user", "content": req.prompt }],
    });
    let resp = Request::post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-dangerous-direct-browser-access", "true")
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| format!("Cannot reach Claude API: {e}"))?;
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Bad Claude response: {e}"))?;
    if let Some(err) = v.get("error") {
        return Err(format!("Claude error: {err}"));
    }
    Ok(v["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string())
}

async fn direct_openai(req: &GenerateRequest) -> Result<String, String> {
    let base = req.base_url.clone().unwrap_or_default();
    if base.is_empty() {
        return Err(format!(
            "Provider \"{}\" has no base URL configured.",
            req.provider
        ));
    }
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));
    let mut messages = Vec::new();
    if let Some(sys) = &req.system {
        if !sys.is_empty() {
            messages.push(json!({ "role": "system", "content": sys }));
        }
    }
    messages.push(json!({ "role": "user", "content": req.prompt }));
    let mut body = json!({ "model": req.model, "messages": messages });
    if let Some(t) = req.temperature {
        body["temperature"] = json!(t);
    }
    if let Some(m) = req.max_tokens {
        body["max_tokens"] = json!(m);
    }
    let mut rb = Request::post(&url);
    if let Some(k) = &req.api_key {
        if !k.is_empty() {
            rb = rb.header("Authorization", &format!("Bearer {k}"));
        }
    }
    let resp = rb
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| {
            format!(
                "{} request failed: Cannot connect to API: {e}",
                req.provider
            )
        })?;
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("{} returned a non-JSON response: {e}", req.provider))?;
    if let Some(err) = v.get("error") {
        return Err(format!("{} error: {err}", req.provider));
    }
    Ok(v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string())
}

// ----- Settings modal -----
#[component]
fn SettingsModal() -> Element {
    let mut state = use_context::<AppState>();
    let mut draft = use_signal(|| state.settings.peek().clone());

    // provider editor draft
    let mut editing = use_signal(|| Option::<ProviderConfig>::None);
    let mut models_text = use_signal(String::new);

    let close = move |_| state.show_settings.set(false);

    let save_all = move |_| {
        let s = draft.peek().clone();
        save_settings(&s);
        state.settings.set(s);
        state.show_settings.set(false);
    };

    let providers = draft.read().providers.clone();
    let commands = draft.read().commands.clone();
    let presets = provider_presets();
    let editing_now = editing.read().clone();

    rsx! {
        div { class: "modal-bg",
            div { class: "modal",
                h2 { "⚙ T.C.K Settings" }

                div { class: "section-title", "Claude API key" }
                div { class: "row",
                    input {
                        r#type: "password",
                        placeholder: "sk-ant-…",
                        value: "{draft.read().claude_api_key}",
                        oninput: move |e| draft.write().claude_api_key = e.value(),
                    }
                }

                div { class: "section-title", "Ollama host" }
                div { class: "row",
                    input {
                        placeholder: "http://localhost:11434",
                        value: "{draft.read().ollama_host}",
                        oninput: move |e| draft.write().ollama_host = e.value(),
                    }
                }

                div { class: "section-title", "OpenAI-compatible providers" }
                for p in providers.iter() {
                    div { key: "{p.id}", class: "provider-item",
                        div {
                            strong { "{p.name}" }
                            br {}
                            code { "{p.base_url}" }
                        }
                        button {
                            onclick: {
                                let id = p.id.clone();
                                move |_| {
                                    draft.write().providers.retain(|x| x.id != id);
                                }
                            },
                            "Remove"
                        }
                    }
                }

                if let Some(ed) = editing_now {
                    div { style: "margin-top:10px;border-top:1px solid var(--border);padding-top:10px;",
                        div { class: "presets",
                            for (name , url) in presets.iter() {
                                button {
                                    key: "{name}",
                                    onclick: {
                                        let name = name.to_string();
                                        let url = url.to_string();
                                        move |_| {
                                            let mut e = editing.peek().clone().unwrap_or_default();
                                            if e.name.is_empty() { e.name = name.clone(); }
                                            e.base_url = url.clone();
                                            editing.set(Some(e));
                                        }
                                    },
                                    "{name}"
                                }
                            }
                        }
                        div { class: "row",
                            label { "Name" }
                            input {
                                value: "{ed.name}",
                                oninput: move |e| {
                                    let mut x = editing.peek().clone().unwrap_or_default();
                                    x.name = e.value();
                                    editing.set(Some(x));
                                },
                            }
                        }
                        div { class: "row",
                            label { "Base URL" }
                            input {
                                placeholder: "https://opencode.ai/zen/v1",
                                value: "{ed.base_url}",
                                oninput: move |e| {
                                    let mut x = editing.peek().clone().unwrap_or_default();
                                    x.base_url = e.value();
                                    editing.set(Some(x));
                                },
                            }
                        }
                        div { class: "row",
                            label { "API key" }
                            input {
                                r#type: "password",
                                value: "{ed.api_key}",
                                oninput: move |e| {
                                    let mut x = editing.peek().clone().unwrap_or_default();
                                    x.api_key = e.value();
                                    editing.set(Some(x));
                                },
                            }
                        }
                        div { class: "row",
                            label { "Models" }
                            input {
                                placeholder: "comma,separated,model,ids",
                                value: "{models_text}",
                                oninput: move |e| models_text.set(e.value()),
                            }
                        }
                        div { class: "right",
                            button {
                                onclick: move |_| {
                                    editing.set(None);
                                    models_text.set(String::new());
                                },
                                "Cancel"
                            }
                            button {
                                class: "primary",
                                onclick: move |_| {
                                    let mut p = editing.peek().clone().unwrap_or_default();
                                    if p.id.is_empty() {
                                        p.id = format!("cp-{}", js_sys::Date::now() as u64);
                                    }
                                    p.models = models_text
                                        .peek()
                                        .split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                    if !p.name.trim().is_empty() && !p.base_url.trim().is_empty() {
                                        draft.write().providers.retain(|x| x.id != p.id);
                                        draft.write().providers.push(p);
                                        editing.set(None);
                                        models_text.set(String::new());
                                    }
                                },
                                "Add provider"
                            }
                        }
                    }
                } else {
                    button {
                        style: "margin-top:8px;",
                        onclick: move |_| {
                            editing.set(Some(ProviderConfig::default()));
                            models_text.set(String::new());
                        },
                        "+ Add provider"
                    }
                }

                div { class: "section-title", style: "margin-top:14px;", "Custom command buttons" }
                for c in commands.iter() {
                    div { key: "{c.id}", class: "provider-item",
                        div { "{c.icon} {c.label} — " code { "{c.command}" } }
                        button {
                            onclick: {
                                let id = c.id.clone();
                                move |_| { draft.write().commands.retain(|x| x.id != id); }
                            },
                            "Remove"
                        }
                    }
                }
                CommandAdder { draft }

                div { class: "right",
                    button { onclick: close, "Cancel" }
                    button { class: "primary", onclick: save_all, "Save" }
                }
            }
        }
    }
}

#[component]
fn CommandAdder(draft: Signal<Settings>) -> Element {
    let mut label = use_signal(String::new);
    let mut command = use_signal(String::new);
    let mut icon = use_signal(|| "▶".to_string());

    rsx! {
        div { class: "row", style: "margin-top:6px;",
            input {
                style: "width:48px;flex:none;",
                value: "{icon}",
                oninput: move |e| icon.set(e.value()),
            }
            input {
                placeholder: "Label",
                value: "{label}",
                oninput: move |e| label.set(e.value()),
            }
            input {
                placeholder: "command to run",
                value: "{command}",
                oninput: move |e| command.set(e.value()),
            }
            button {
                onclick: move |_| {
                    let l = label.peek().clone();
                    let c = command.peek().clone();
                    if !l.trim().is_empty() && !c.trim().is_empty() {
                        draft.write().commands.push(CustomCommand {
                            id: format!("cmd-{}", js_sys::Date::now() as u64),
                            label: l,
                            command: c,
                            icon: icon.peek().clone(),
                        });
                        label.set(String::new());
                        command.set(String::new());
                    }
                },
                "Add"
            }
        }
    }
}
