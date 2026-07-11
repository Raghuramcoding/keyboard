use dioxus::prelude::*;
use crate::state::AppState;
use tck_core::{ProviderConfig, provider_presets, Settings, CustomCommand};

#[component]
pub fn SettingsModal() -> Element {
    let mut state = use_context::<AppState>();
    let mut draft = use_signal(|| state.settings.peek().clone());

    // provider editor draft
    let mut editing = use_signal(|| Option::<ProviderConfig>::None);
    let mut models_text = use_signal(String::new);

    let close = move |_| state.show_settings.set(false);

    let save_all = move |_| {
        let s = draft.peek().clone();
        crate::state::save_settings(&s);
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
                            for (name, url) in presets.iter() {
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