use dioxus::prelude::*;
use crate::state::AppState;
use crate::api::{fetch_repos, fetch_contents};
use gloo_net::http::Request;
use tck_core::{GitHubStatus, RepoEntry};

#[component]
pub fn SourceControl() -> Element {
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

fn load_repos(state: AppState) {
    let mut repos = state.repos;
    let mut git_msg = state.git_msg;
    let mut browse_repo = state.browse_repo;
    git_msg.set("Loading repos…".to_string());
    crate::spawn(async move {
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
    crate::spawn(async move {
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
        crate::spawn(async move {
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
    crate::spawn(async move {
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