//! T.C.K — TalentCloud Keyboard
//! A pure-Rust (Dioxus/WASM) AI coding environment that runs in the browser.

mod instance_dashboard;
mod landing;
mod orchestrator;

use dioxus::prelude::*;
use futures_util::{SinkExt, StreamExt};
use gloo_net::http::Request;
use gloo_net::websocket::{futures::WebSocket, Message as WsMessage};
use gloo_storage::{LocalStorage, Storage};
use landing::LandingPage;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use instance_dashboard::{InstancePanel, InstanceSidebar};
use tck_core::{
    agent_color, bucket_agent, claude_models, known_agents, provider_presets, scaffold_templates,
    usage_pct, AgentTokenUsage, CommitRequest, CustomCommand, EventMessage, GenerateRequest,
    GenerateResponse, GitHubStatus, ProviderConfig, RepoEntry, RepoFile, RepoInfo,
    ScaffoldToRepoRequest, Settings,
};
use wasm_bindgen::JsCast;

/// A model from the models.dev catalog, returned by tck-ai.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CatalogModel {
    id: String,
    provider: String,
    name: String,
    context_window: usize,
    #[serde(default)]
    supports_tools: bool,
    #[serde(default)]
    supports_streaming: bool,
}

const SETTINGS_KEY: &str = "tck.settings";

fn main() {
    dioxus::launch(App);
}

// ----- chat -----
#[derive(Clone, PartialEq, Serialize, Deserialize)]
enum Role {
    User,
    Ai,
    Err,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct ChatMsg {
    role: Role,
    text: String,
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
    /// Token/character consumption per agent tool across the entire session.
    token_usage: Signal<Vec<AgentTokenUsage>>,
    /// True once a native T.C.K server is detected (via /api/health). When false
    /// (e.g. served from GitHub Pages) AI calls go directly to the provider.
    server: Signal<bool>,
    /// Model catalog fetched from models.dev via tck-ai.
    model_catalog: Signal<Vec<CatalogModel>>,
    // ----- Multi-instance dashboard -----
    view: Signal<instance_dashboard::AppView>,
    instances: Signal<Vec<instance_dashboard::InstanceState>>,
    active_id: Signal<Option<String>>,
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

/// WebSocket URL for the instance-events stream (agent state transitions, etc.).
fn events_ws_url() -> String {
    let loc = web_sys::window().unwrap().location();
    let proto = if loc.protocol().unwrap_or_default() == "https:" { "wss" } else { "ws" };
    let host = loc.host().unwrap_or_else(|_| "127.0.0.1:8080".into());
    format!("{proto}://{host}/ws/events")
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
    let token_usage = use_signal(Vec::<AgentTokenUsage>::new);
    let model_catalog = use_signal(Vec::<CatalogModel>::new);
    // ---- Multi-instance dashboard state ----
    let view = use_signal(|| instance_dashboard::AppView::Dashboard);
    let instances = use_signal(|| {
        // Restore persisted instances from localStorage.
        LocalStorage::get::<String>("tck.instances")
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<instance_dashboard::InstanceState>>(&s).ok())
            .unwrap_or_default()
    });
    let active_id = use_signal(|| Option::<String>::None);

    // Detect a native T.C.K server. On a static host (GitHub Pages) this 404s and
    // we stay in "serverless" mode: AI calls go straight to the provider.
    let mut gh_w = gh;
    use_future(move || async move {
        let has_server = if let Ok(resp) = Request::get("/api/health").send().await {
            if resp.ok() {
                if let Ok(t) = resp.text().await {
                    t.trim() == "ok"
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };
        server.set(has_server);

        // Also learn GitHub status (configured / already connected).
        if let Some(s) = fetch_gh_status().await {
            gh_w.set(s);
        }
    });

    // Fetch model catalog from tck-ai (models.dev backed).
    let mut mc = model_catalog;
    use_future(move || async move {
        if let Ok(resp) = Request::get("/api/ai/models").send().await {
            if let Ok(models) = resp.json::<Vec<CatalogModel>>().await {
                mc.set(models);
            }
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
        token_usage,
        server,
        gh,
        repos,
        browse_repo,
        browse_path,
        entries,
        open_file,
        git_msg,
        model_catalog,
        view,
        instances,
        active_id,
    });

    // Persist instances to localStorage whenever they change.
    let instances_w = state.instances;
    use_effect(move || {
        let data = instances_w.peek();
        if let Ok(json) = serde_json::to_string(&*data) {
            let _ = LocalStorage::set("tck.instances", &json);
        }
    });

    // Subscribe to server-sent events (agent state transitions, liveness, etc.)
    // via a persistent WebSocket that auto-reconnects on disconnect.
    {
        let instances = state.instances;
        let server_signaled = state.server;
        use_future(move || async move {
            loop {
                if !*server_signaled.peek() {
                    gloo_timers::future::TimeoutFuture::new(1000).await;
                    continue;
                }
                match WebSocket::open(&events_ws_url()) {
                    Ok(ws) => {
                        let (_write, mut read) = ws.split();
                        while let Some(item) = read.next().await {
                            match item {
                                Ok(WsMessage::Text(text)) => {
                                    if let Ok(event) = serde_json::from_str::<EventMessage>(&text) {
                                        match event {
                                            EventMessage::AgentStateChanged {
                                                instance_id,
                                                new_state,
                                                ..
                                            } => {
                                                let mut list = instances.peek().clone();
                                                if let Some(inst) =
                                                    list.iter_mut().find(|s| s.info.id == instance_id)
                                                {
                                                    inst.agent_state = new_state;
                                                    instances.set(list);
                                                }
                                            }
                                            EventMessage::InstanceStatusChanged {
                                                instance_id,
                                                status,
                                                ..
                                            } => {
                                                let mut list = instances.peek().clone();
                                                if let Some(inst) =
                                                    list.iter_mut().find(|s| s.info.id == instance_id)
                                                {
                                                    inst.info.status = status;
                                                    instances.set(list);
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Ok(WsMessage::Bytes(_)) => {}
                                Err(_) => break,
                            }
                        }
                    }
                    Err(_) => {}
                }
                gloo_timers::future::TimeoutFuture::new(3000).await;
            }
        });
    }

    rsx! {
        div { class: "app",
            TitleBar {}
            InstanceDashboard {}
            if *state.show_settings.read() {
                SettingsModal {}
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
    let catalog = state.model_catalog.read().clone();
    let suggestions = model_suggestions(&provider_now, &settings, &catalog);

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

// ---- Instance Dashboard ----

#[component]
fn InstanceDashboard() -> Element {
    let mut state = use_context::<AppState>();

    // Fetch the instance list from the server on mount.
    use_future(move || async move {
        let list = instance_dashboard::fetch_instances().await;
        let mut instances = state.instances;
        let instates: Vec<instance_dashboard::InstanceState> = list
            .into_iter()
            .map(|info| {
                let agent_state = info.agent_state;
                instance_dashboard::InstanceState {
                    info,
                    term_lines: String::new(),
                    term_connected: false,
                    messages: Vec::new(),
                    agent_state,
                }
            })
            .collect();
        // Auto-select the first instance if none selected.
        if !instates.is_empty() && state.active_id.peek().is_none() {
            state.active_id.set(Some(instates[0].info.id.clone()));
        }
        instances.set(instates);
    });

    let instances_list = state.instances.read().clone();
    let active = state.active_id.read().clone();
    let selected = instances_list.iter().find(|s| Some(s.info.id.as_str()) == active.as_deref()).cloned();

    rsx! {
        div { class: "dashboard",
            InstanceSidebar {
                instances: instances_list.clone(),
                active_id: active.clone(),
                on_select: move |id| { state.active_id.set(Some(id)); },
                on_create_terminal: move |cmd: String| {
                    spawn(async move {
                        let req = tck_core::CreateInstanceRequest {
                            kind: tck_core::InstanceKind::Terminal,
                            label: Some(cmd.clone()),
                            command: Some(cmd.clone()),
                            cwd: None,
                        };
                        if let Some(info) = instance_dashboard::create_instance(&req).await {
                            let mut list = state.instances.peek().clone();
                            let agent_state = info.agent_state;
                            list.push(instance_dashboard::InstanceState {
                                info: info.clone(),
                                term_lines: String::new(),
                                term_connected: false,
                                messages: Vec::new(),
                                agent_state,
                            });
                            state.instances.set(list);
                            let id = info.id.clone();
                            state.active_id.set(Some(id.clone()));
                            // Connect WebSocket for terminal output.
                            connect_instance_terminal(state, &id);
                        }
                    });
                },
                on_create_chat: move |_| {
                    let id = format!("chat_{:016x}", rand_u64());
                    let info = tck_core::InstanceInfo {
                        id: id.clone(),
                        kind: tck_core::InstanceKind::Chat,
                        label: "AI Chat".into(),
                        command: None,
                        cwd: None,
                        status: tck_core::InstanceStatus::Running,
                        error: None,
                        created_at: js_sys::Date::now() as u64,
                    };
                    let mut list = state.instances.peek().clone();
                    let agent_state = info.agent_state;
                    list.push(instance_dashboard::InstanceState {
                        info: info.clone(),
                        term_lines: String::new(),
                        term_connected: false,
                        messages: Vec::new(),
                        agent_state,
                    });
                    state.instances.set(list);
                    state.active_id.set(Some(id));
                },
                on_delete: move |id: String| {
                    let id_c = id.clone();
                    spawn(async move {
                        instance_dashboard::delete_instance(&id_c).await;
                        let mut list = state.instances.peek().clone();
                        list.retain(|s| s.info.id != id_c);
                        let new_active = if state.active_id.peek().as_deref() == Some(&id_c) {
                            list.first().map(|s| s.info.id.clone())
                        } else {
                            state.active_id.peek().clone()
                        };
                        state.instances.set(list);
                        state.active_id.set(new_active);
                    });
                },
            }
            InstancePanel {
                instance: selected.clone(),
                on_send: move |args: (String, String)| {
                    send_to_instance(state, &args.0, &args.1);
                },
            }
        }
    }
}

fn connect_instance_terminal(mut state: AppState, id: &str) {
    let id = id.to_string();
    let ws_url = instance_dashboard::instance_ws_url(&id);

    spawn(async move {
        match WebSocket::open(&ws_url) {
            Ok(ws) => {
                let (_write, mut read) = ws.split();
                let mut received = false;
                while let Some(item) = read.next().await {
                    let msg = match item {
                        Ok(m) => m,
                        Err(_) => break,
                    };
                    let text = match msg {
                        WsMessage::Text(t) => t,
                        WsMessage::Bytes(b) => String::from_utf8_lossy(&b).to_string(),
                    };
                    let sanitized = sanitize_terminal(&text);
                    let mut list = state.instances.peek().clone();
                    if let Some(inst) = list.iter_mut().find(|s| s.info.id == id) {
                        if !received {
                            received = true;
                            inst.term_connected = true;
                        }
                        inst.term_lines.push_str(&sanitized);
                        if inst.term_lines.len() > 120_000 {
                            inst.term_lines = inst.term_lines.split_off(inst.term_lines.len() - 100_000);
                        }
                    }
                    state.instances.set(list);
                }
                // Stream ended / disconnected.
                let mut list = state.instances.peek().clone();
                if let Some(inst) = list.iter_mut().find(|s| s.info.id == id) {
                    inst.term_connected = false;
                    if !received {
                        inst.term_lines.push_str("[T.C.K] Terminal offline — cannot reach server.\n");
                    }
                }
                state.instances.set(list);
            }
            Err(_) => {
                let mut list = state.instances.peek().clone();
                if let Some(inst) = list.iter_mut().find(|s| s.info.id == id) {
                    inst.term_connected = false;
                    inst.term_lines.push_str("[T.C.K] Failed to connect to terminal.\n");
                }
                state.instances.set(list);
            }
        }
    });
}

fn send_to_instance(mut state: AppState, id: &str, text: &str) {
    let id_str = id.to_string();
    let text_str = text.to_string();
    spawn(async move {
        // Send input to terminal instance (if a server-side instance).
        instance_dashboard::write_stdin(&id_str, &text_str).await;

        let mut list = state.instances.peek().clone();
        let kind = list.iter().find(|s| s.info.id == id_str).map(|s| s.info.kind);

        if kind == Some(tck_core::InstanceKind::Terminal) {
            // Terminal: just echo input locally.
            if let Some(inst) = list.iter_mut().find(|s| s.info.id == id_str) {
                inst.term_lines.push_str(&format!("$ {text_str}\n"));
            }
            state.instances.set(list);
            return;
        }

        // Chat instance: push user message immediately.
        if let Some(inst) = list.iter_mut().find(|s| s.info.id == id_str) {
            inst.messages.push(ChatMsg {
                role: Role::User,
                text: text_str.clone(),
            });
        }
        state.instances.set(list);

        // Build and send AI generation request.
        let req = build_request(&state, text_str);
        let has_server = *state.server.peek();
        let provider_id = state.provider.peek().clone();
        let resp = call_generate(req, has_server).await;

        let mut list2 = state.instances.peek().clone();
        if let Some(inst) = list2.iter_mut().find(|s| s.info.id == id_str) {
            let msg = match (resp.response, resp.error) {
                (Some(r), _) => ChatMsg { role: Role::Ai, text: r },
                (_, Some(e)) => ChatMsg { role: Role::Err, text: e },
                _ => ChatMsg { role: Role::Err, text: "Empty response from provider.".into() },
            };
            let chars = msg.text.len();
            inst.messages.push(msg);
            state.instances.set(list2);
            record_usage(state.token_usage, &provider_id, chars);
        }
    });
}

fn rand_u64() -> u64 {
    let d = js_sys::Date::now();
    ((d * 1_000_000.0) as u64) ^ (web_sys::window().map(|w| w.inner_width().unwrap_or_default().as_f64().unwrap_or(0.0) as u64).unwrap_or(0))
}

fn model_suggestions(provider: &str, settings: &Settings, catalog: &[CatalogModel]) -> Vec<String> {
    // First, collect static suggestions.
    let mut suggestions: Vec<String> = match provider {
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
    };
    // Then merge in models from the catalog that match this provider.
    for cm in catalog.iter() {
        if cm.provider == provider && !suggestions.contains(&cm.id) {
            // Strip the "provider/" prefix from the SDK model id for display.
            let display = cm.id.strip_prefix(&format!("{provider}/")).unwrap_or(&cm.id);
            suggestions.push(display.to_string());
        }
    }
    suggestions
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

/// Record character output produced by an agent/tool into the shared token usage tracker.
fn record_usage(mut usage: Signal<Vec<AgentTokenUsage>>, agent_id: &str, chars: usize) {
    let (bucket, icon) = bucket_agent(agent_id);
    let mut list = usage.peek().clone();
    if let Some(existing) = list.iter_mut().find(|u| u.agent_id == bucket) {
        existing.merge(chars);
    } else {
        let name = known_agents()
            .into_iter()
            .find(|a| a.id == bucket)
            .map(|a| a.label)
            .unwrap_or_else(|| {
                // Capitalise first letter as a fallback name.
                let mut c = bucket.chars();
                c.next()
                    .map(|f| f.to_uppercase().to_string() + c.as_str())
                    .unwrap_or_default()
            });
        let mut u = AgentTokenUsage {
            agent_id: bucket.to_string(),
            name,
            icon: icon.to_string(),
            ..Default::default()
        };
        u.merge(chars);
        list.push(u);
    }
    usage.set(list);
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

/// The right-hand panel hosts two tools: the single-model AI Chat and the
/// multi-agent Orchestrator. Both stay mounted (state survives tab switches);
/// the inactive one is hidden with CSS.
#[derive(Clone, Copy, PartialEq)]
enum RightTab {
    Chat,
    Orchestrate,
}

#[component]
fn RightPanel() -> Element {
    let state = use_context::<AppState>();
    let mut tab = use_signal(|| RightTab::Orchestrate);
    let t = *tab.read();
    rsx! {
        div { class: "chat",
            TokenUsageDashboard { token_usage: state.token_usage }
            div { class: "right-tabs",
                button {
                    class: if t == RightTab::Chat { "rtab active" } else { "rtab" },
                    onclick: move |_| tab.set(RightTab::Chat),
                    "🤖 AI Chat"
                }
                button {
                    class: if t == RightTab::Orchestrate { "rtab active" } else { "rtab" },
                    onclick: move |_| tab.set(RightTab::Orchestrate),
                    "🎭 Orchestrator"
                }
            }
            div { class: if t == RightTab::Chat { "right-body" } else { "right-body hidden" },
                ChatPanel {}
            }
            div { class: if t == RightTab::Orchestrate { "right-body" } else { "right-body hidden" },
                orchestrator::OrchestratorPanel {}
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
        let provider_id = state.provider.peek().clone();
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
            let chars = msg.text.len();
            state.chat.write().push(msg);
            state.busy.set(false);
            record_usage(state.token_usage, &provider_id, chars);
        });
    };

    rsx! {
        div { class: "chat-body",
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

// ===================== Token usage dashboard =====================

/// Vibrant donut chart + agent-usage table showing which AI tools consume the most
/// tokens across the whole session.  Always visible at the top of the right panel.
#[component]
fn TokenUsageDashboard(token_usage: Signal<Vec<AgentTokenUsage>>) -> Element {
    let agents = token_usage.read().clone();
    let total_chars: usize = agents.iter().map(|a| a.chars_produced).sum();
    let total_tokens: usize = total_chars / 4;

    let mut collapsed = use_signal(|| true);

    // Pre-compute legend rows so we don't mix complex Rust expressions inside rsx.
    let legend_rows: Vec<LegendRow> = agents
        .iter()
        .map(|a| {
            let pct_str = usage_pct(a.chars_produced, total_chars);
            let pct_num = pct_str.trim_end_matches('%').to_string();
            let color = agent_color(&a.agent_id).to_string();
            LegendRow {
                icon: a.icon.clone(),
                name: a.name.clone(),
                tokens: a.tokens,
                color,
                pct_str,
                pct_num,
                bar_width: if total_chars > 0 {
                    100.0 * a.chars_produced as f64 / total_chars as f64
                } else {
                    0.0
                },
            }
        })
        .collect();

    rsx! {
        div { class: "tkd",
            div { class: "tkd-header",
                div { class: "tkd-header-left",
                    span { class: "tkd-icon", "⚡" }
                    div {
                        div { class: "tkd-title", "Token Dashboard" }
                        div { class: "tkd-sub", "{total_tokens} tot · {agents.len()} tools" }
                    }
                }
                button {
                    class: "tkd-toggle",
                    onclick: move |_| {
                        let cur = *collapsed.peek();
                        collapsed.set(!cur);
                    },
                    if *collapsed.read() { "▸" } else { "▾" }
                }
            }
            if !*collapsed.read() && !agents.is_empty() {
                div { class: "tkd-body",
                    div { class: "tkd-donut-wrap",
                        DonutChart { agents: agents.clone(), total_chars }
                    }
                    div { class: "tkd-legend",
                        for row in legend_rows.iter() {
                            div { class: "tkd-legend-item",
                                span {
                                    class: "tkd-legend-dot",
                                    style: format!("background:{};", row.color),
                                }
                                span { class: "tkd-legend-name",
                                    "{row.icon} {row.name}"
                                }
                                span { class: "tkd-legend-bar-wrap",
                                    span {
                                        class: "tkd-legend-bar",
                                        style: format!("width:{}%;background:{};", row.pct_num, row.color),
                                    }
                                }
                                span { class: "tkd-legend-pct", "{row.pct_str}" }
                                span { class: "tkd-legend-abs", "{row.tokens} tok" }
                            }
                        }
                    }
                }
            }
            if *collapsed.read() && !agents.is_empty() {
                div { class: "tkd-bar-row",
                        for row in legend_rows.iter() {
                            div {
                                class: "tkd-bar-mini",
                                title: format!("{}: {} tokens ({})", row.name, row.tokens, row.pct_str),
                                style: format!("flex:{};background:{};", row.bar_width.max(1.0) as usize, row.color),
                            }
                        }
                }
            }
        }
    }
}

#[derive(Clone)]
struct LegendRow {
    icon: String,
    name: String,
    tokens: usize,
    color: String,
    pct_str: String,
    pct_num: String,
    bar_width: f64,
}

/// SVG donut chart — one arc per tracked agent, sized by token consumption.
#[component]
fn DonutChart(agents: Vec<AgentTokenUsage>, total_chars: usize) -> Element {
    let radius = 66.0;
    let stroke_w = 26.0;
    let circumference = 2.0 * std::f64::consts::PI * radius;

    // Pre-compute all arcs before entering rsx! (Dioxus for-body can't execute
    // arbitrary statements between elements).
    struct ArcSeg {
        key: String,
        color: String,
        dasharray: String,
        dashoffset: String,
        tooltip: String,
    }
    let gap = 2.0;
    let mut cumulative = 0.0;
    let mut arcs: Vec<ArcSeg> = agents
        .iter()
        .map(|a| {
            let p = if total_chars > 0 {
                a.chars_produced as f64 / total_chars as f64
            } else {
                0.0
            };
            let seg_len = circumference * p;
            let draw = (seg_len - gap).max(0.0);
            let rest = circumference - draw;
            let offset = cumulative;
            cumulative += seg_len;
            let pct_str = usage_pct(a.chars_produced, total_chars);
            ArcSeg {
                key: a.agent_id.clone(),
                color: agent_color(&a.agent_id).to_string(),
                dasharray: format!("{draw:.1} {rest:.1}"),
                dashoffset: format!("{:.1}", -offset),
                tooltip: format!("{}: {} tokens ({})", a.name, a.tokens, pct_str),
            }
        })
        .collect();
    // Largest arc first for best visual layering.
    arcs.sort_by(|a, b| {
        b.dasharray
            .len()
            .partial_cmp(&a.dasharray.len())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    rsx! {
        div { class: "tkd-donut",
            svg { view_box: "0 0 200 200",
                circle { cx: "100", cy: "100", r: "{radius}",
                    fill: "none", stroke: "#2d2d30", "stroke-width": "{stroke_w}" }
                for s in arcs.iter() {
                    circle {
                        key: "{s.key}",
                        cx: "100", cy: "100", r: "{radius}",
                        fill: "none",
                        stroke: "{s.color}",
                        "stroke-width": "{stroke_w}",
                        "stroke-dasharray": "{s.dasharray}",
                        "stroke-dashoffset": "{s.dashoffset}",
                        "stroke-linecap": "round",
                        transform: "rotate(-90 100 100)",
                        title { "{s.tooltip}" }
                    }
                }
                text {
                    x: "100", y: "92",
                    text_anchor: "middle",
                    fill: "#d4d4d4",
                    "font-size": "18",
                    "font-weight": "700",
                    "font-family": "system-ui, sans-serif",
                    "{format_tk(total_chars / 4)}"
                }
                text {
                    x: "100", y: "112",
                    text_anchor: "middle",
                    fill: "#858585",
                    "font-size": "11",
                    "font-family": "system-ui, sans-serif",
                    "tokens across {agents.len()} agents"
                }
            }
        }
    }
}

/// Format a number with K/M suffix for the center of the donut.
fn format_tk(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Like `build_request` but with a caller-supplied system prompt (used by the
/// orchestrator to give each agent its role).
fn build_request_sys(state: &AppState, prompt: String, system: String) -> GenerateRequest {
    let mut req = build_request(state, prompt);
    req.system = Some(system);
    req
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
    let built = match Request::post("/api/ai/generate").json(&req) {
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

use std::sync::OnceLock;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

// ---- Cached references to the global TckAI object so we only reflect once.
struct TckAIBridge {
    namespace: JsValue,
    generate_fn: JsValue,
}

static TCKAI: OnceLock<Option<TckAIBridge>> = OnceLock::new();

/// Try to initialise the TckAI bridge by finding `window.TckAI` and caching
/// its `.generate` method.  Returns `None` if the bundle isn't loaded.
fn init_tck_ai() -> Option<&'static TckAIBridge> {
    TCKAI.get_or_init(|| {
        let win = web_sys::window()?;
        let ns = js_sys::Reflect::get(&win, &JsValue::from_str("TckAI")).ok()?;
        if ns.is_undefined() || ns.is_null() { return None; }
        let gen = js_sys::Reflect::get(&ns, &JsValue::from_str("generate")).ok()?;
        Some(TckAIBridge { namespace: ns, generate_fn: gen })
    }).as_ref()
}

/// Call `window.TckAI.generate(req)` via JS interop.
/// Returns `None` if the TckAI bundle isn't loaded or the call fails.
async fn generate_via_bridge(req: &GenerateRequest) -> Option<GenerateResponse> {
    let bridge = init_tck_ai()?;

    // Convert the Rust GenerateRequest to a plain JS object.
    let js_req = js_sys::Object::new();
    js_sys::Reflect::set(&js_req, &JsValue::from_str("provider"), &JsValue::from_str(&req.provider)).ok()?;
    js_sys::Reflect::set(&js_req, &JsValue::from_str("model"), &JsValue::from_str(&req.model)).ok()?;
    js_sys::Reflect::set(&js_req, &JsValue::from_str("prompt"), &JsValue::from_str(&req.prompt)).ok()?;
    if let Some(sys) = &req.system {
        js_sys::Reflect::set(&js_req, &JsValue::from_str("system"), &JsValue::from_str(sys)).ok()?;
    }
    if let Some(k) = &req.api_key {
        js_sys::Reflect::set(&js_req, &JsValue::from_str("apiKey"), &JsValue::from_str(k)).ok()?;
    }
    if let Some(b) = &req.base_url {
        js_sys::Reflect::set(&js_req, &JsValue::from_str("baseUrl"), &JsValue::from_str(b)).ok()?;
    }
    if let Some(t) = req.temperature {
        js_sys::Reflect::set(&js_req, &JsValue::from_str("temperature"), &JsValue::from_f64(t as f64)).ok()?;
    }
    if let Some(m) = req.max_tokens {
        js_sys::Reflect::set(&js_req, &JsValue::from_str("maxTokens"), &JsValue::from_f64(m as f64)).ok()?;
    }

    // Call TckAI.generate(js_req) and await the promise.
    let gen_fn = bridge.generate_fn.dyn_ref::<js_sys::Function>()?;
    let promise = gen_fn.call1(&bridge.namespace, &js_req).ok()?;
    let promise = promise.dyn_into::<js_sys::Promise>().ok()?;
    let result = JsFuture::from(promise).await.ok()?;

    // Parse the result object back into a GenerateResponse.
    let response = js_sys::Reflect::get(&result, &JsValue::from_str("response"))
        .ok()?.as_string().unwrap_or_default();
    let model = js_sys::Reflect::get(&result, &JsValue::from_str("model"))
        .ok()?.as_string().unwrap_or_default();

    Some(GenerateResponse {
        response: Some(response),
        model: Some(model),
        ..Default::default()
    })
}

/// Serverless mode: call the AI provider straight from the browser. Works for
/// CORS-friendly providers (Anthropic with the browser-access header, OpenRouter,
/// a local Ollama with `OLLAMA_ORIGINS=*`, etc.).
///
/// If the TckAI browser bundle (tck-ai.js) is loaded, it uses the Vercel AI SDK
/// for unified provider access. Otherwise falls back to raw HTTP calls.
async fn generate_direct(req: GenerateRequest) -> GenerateResponse {
    // First try the TckAI browser bundle (Vercel AI SDK).
    if let Some(resp) = generate_via_bridge(&req).await {
        return resp;
    }
    // Fall back to direct HTTP calls.
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
