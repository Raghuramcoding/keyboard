use dioxus::prelude::*;
use gloo_storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};
use tck_core::{AgentTokenUsage, Settings, InstanceInfo, AgentState, known_agents, bucket_agent};

const SETTINGS_KEY: &str = "tck.settings";
const INSTANCES_KEY: &str = "tck.instances";

#[derive(Clone, Copy)]
pub struct AppState {
    pub settings: Signal<Settings>,
    pub code: Signal<String>,
    pub filename: Signal<String>,
    pub provider: Signal<String>,
    pub model: Signal<String>,
    pub chat: Signal<Vec<tck_core::ChatMsg>>,
    pub busy: Signal<bool>,
    pub show_settings: Signal<bool>,
    pub term_lines: Signal<String>,
    pub term_connected: Signal<bool>,
    pub term: Coroutine<String>,
    pub token_usage: Signal<Vec<AgentTokenUsage>>,
    pub server: Signal<bool>,
    pub gh: Signal<tck_core::GitHubStatus>,
    pub repos: Signal<Vec<tck_core::RepoInfo>>,
    pub browse_repo: Signal<Option<String>>,
    pub browse_path: Signal<String>,
    pub entries: Signal<Vec<tck_core::RepoEntry>>,
    pub open_file: Signal<Option<(String, String, String)>>,
    pub git_msg: Signal<String>,
    pub model_catalog: Signal<Vec<tck_core::CatalogModel>>,
    pub view: Signal<AppView>,
    pub instances: Signal<Vec<InstanceState>>,
    pub active_id: Signal<Option<String>>,
}

impl AppState {
    pub fn new(term: Coroutine<String>) -> Self {
        Self {
            settings: Signal::new(load_settings()),
            code: Signal::new("// Welcome to T.C.K — TalentCloud Keyboard\n// A pure-Rust AI coding environment running in your browser.\n\nfn main() {\n    println!(\"Hello from T.C.K!\");\n}\n".to_string()),
            filename: Signal::new("scratch.rs".to_string()),
            provider: Signal::new("ollama".to_string()),
            model: Signal::new(String::new()),
            chat: Signal::new(Vec::new()),
            busy: Signal::new(false),
            show_settings: Signal::new(false),
            term_lines: Signal::new(String::new()),
            term_connected: Signal::new(false),
            token_usage: Signal::new(Vec::new()),
            server: Signal::new(false),
            gh: Signal::new(tck_core::GitHubStatus::default()),
            repos: Signal::new(Vec::new()),
            browse_repo: Signal::new(None),
            browse_path: Signal::new(String::new()),
            entries: Signal::new(Vec::new()),
            open_file: Signal::new(None),
            git_msg: Signal::new(String::new()),
            model_catalog: Signal::new(Vec::new()),
            view: Signal::new(AppView::Dashboard),
            instances: Signal::new(load_instances()),
            active_id: Signal::new(None),
            term: term,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AppView {
    Dashboard,
    Editor,
    Orchestrator,
}

impl Default for AppView {
    fn default() -> Self { AppView::Dashboard }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceState {
    pub info: InstanceInfo,
    pub term_lines: String,
    pub term_connected: bool,
    pub messages: Vec<tck_core::ChatMsg>,
    pub agent_state: AgentState,
}

fn load_settings() -> Settings {
    LocalStorage::get(SETTINGS_KEY).unwrap_or_default()
}

pub fn save_settings(s: &Settings) {
    let _ = LocalStorage::set(SETTINGS_KEY, s);
}

fn load_instances() -> Vec<InstanceState> {
    LocalStorage::get::<String>(INSTANCES_KEY)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<InstanceState>>(&s).ok())
        .unwrap_or_default()
}

pub fn record_usage(mut usage: Signal<Vec<AgentTokenUsage>>, agent_id: &str, chars: usize) {
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
                let mut c = bucket.chars();
                c.next().map(|f| f.to_uppercase().to_string() + c.as_str()).unwrap_or_default()
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