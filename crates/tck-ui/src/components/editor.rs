use dioxus::prelude::*;
use crate::state::AppState;
use tck_core::CommitRequest;

#[component]
pub fn Editor() -> Element {
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
    crate::spawn(async move {
        match crate::api::commit_file(&req).await {
            Ok(new_sha) => {
                open_file.set(Some((req.repo, req.path, new_sha)));
                git_msg.set("✓ Committed.".to_string());
            }
            Err(e) => git_msg.set(format!("Commit failed: {e}")),
        }
    });
}