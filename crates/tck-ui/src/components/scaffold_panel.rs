use dioxus::prelude::*;
use crate::state::AppState;
use crate::api::scaffold_repo;
use tck_core::scaffold_templates;

#[component]
pub fn ScaffoldPanel() -> Element {
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
    let req = tck_core::ScaffoldToRepoRequest {
        template,
        repo_name,
        private: false,
    };
    crate::spawn(async move {
        match scaffold_repo(&req).await {
            Ok(url) => git_msg.set(format!("✓ Created: {url}")),
            Err(e) => git_msg.set(format!("Failed: {e}")),
        }
    });
}