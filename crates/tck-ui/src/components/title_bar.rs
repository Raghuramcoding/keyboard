use dioxus::prelude::*;
use crate::state::AppState;
use tck_core::{Settings, CatalogModel, claude_models};

#[component]
pub fn TitleBar() -> Element {
    let mut state = use_context::<AppState>();
    let settings = state.settings.read().clone();

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
                for (id, label) in options.iter() {
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

fn model_suggestions(provider: &str, settings: &Settings, catalog: &[CatalogModel]) -> Vec<String> {
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
    for cm in catalog.iter() {
        if cm.provider == provider && !suggestions.contains(&cm.id) {
            let display = cm.id.strip_prefix(&format!("{provider}/")).unwrap_or(&cm.id);
            suggestions.push(display.to_string());
        }
    }
    suggestions
}