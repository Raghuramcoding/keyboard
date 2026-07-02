//! Multi-instance dashboard — create, switch between, and dispatch to
//! multiple coding tool instances (terminal-based CLI tools + AI chat sessions).

use crate::ChatMsg;
use dioxus::prelude::*;
use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use tck_core::{CreateInstanceRequest, InstanceInfo, InstanceKind, InstanceStatus};

/// Which top-level view is shown.
#[derive(Clone, Copy, PartialEq)]
pub enum AppView {
    Dashboard,
    Editor,
    Orchestrator,
}

/// One running or saved instance, with its UI-relevant state.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct InstanceState {
    pub info: InstanceInfo,
    /// Terminal output buffer (for terminal instances).
    pub term_lines: String,
    pub term_connected: bool,
    /// Chat messages (for chat instances).
    pub messages: Vec<ChatMsg>,
}

// -----------------------------------------------------------------------
// API helpers

/// Fetch the list of instances from the server.
pub async fn fetch_instances() -> Vec<InstanceInfo> {
    let resp = Request::get("/api/instances").send().await;
    match resp {
        Ok(r) => r.json::<Vec<InstanceInfo>>().await.unwrap_or_default(),
        Err(_) => vec![],
    }
}

/// Create a new instance on the server.
pub async fn create_instance(req: &CreateInstanceRequest) -> Option<InstanceInfo> {
    let body = serde_json::to_string(req).ok()?;
    let request = Request::post("/api/instances")
        .header("Content-Type", "application/json")
        .body(body);
    let resp = match request {
        Ok(r) => r.send().await,
        Err(_) => return None,
    };
    match resp {
        Ok(r) => r.json::<InstanceInfo>().await.ok(),
        Err(_) => None,
    }
}

/// Write stdin to a terminal instance.
pub async fn write_stdin(id: &str, input: &str) -> bool {
    let body = serde_json::json!({"input": input}).to_string();
    let req = Request::post(&format!("/api/instances/{id}/stdin"))
        .header("Content-Type", "application/json")
        .body(body);
    let resp = match req {
        Ok(r) => r.send().await,
        Err(_) => return false,
    };
    resp.map(|r| r.ok()).unwrap_or(false)
}

/// Delete an instance on the server.
pub async fn delete_instance(id: &str) -> bool {
    let resp = Request::delete(&format!("/api/instances/{id}"))
        .send()
        .await;
    resp.map(|r| r.ok()).unwrap_or(false)
}

// -----------------------------------------------------------------------
// Sidebar component

/// Sidebar listing all instances with status, plus a "+ New" button.
#[component]
pub fn InstanceSidebar(
    instances: Vec<InstanceState>,
    active_id: Option<String>,
    on_select: EventHandler<String>,
    on_create_terminal: EventHandler<String>,
    on_create_chat: EventHandler<()>,
    on_delete: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "instance-sidebar",
            div { class: "sidebar-section-title", "Instances" }
            div { class: "instance-list",
                for s in instances.iter() {
                    InstanceItem {
                        state: s.clone(),
                        is_active: active_id.as_deref() == Some(s.info.id.as_str()),
                        on_select: on_select,
                        on_delete: on_delete,
                    }
                }
            }
            div { class: "instance-new-group",
                div { class: "section-title-sm", "Launch" }
                div { class: "btn-col",
                    button { onclick: move |_| on_create_terminal.call("claude".into()), "⚡ Claude Code" }
                    button { onclick: move |_| on_create_terminal.call("aider".into()), "🛠 Aider" }
                    button { onclick: move |_| on_create_terminal.call("codex".into()), "🧩 Codex" }
                    button { onclick: move |_| on_create_terminal.call("opencode".into()), "📟 OpenCode" }
                    button { onclick: move |_| on_create_terminal.call("gemini".into()), "✦ Gemini CLI" }
                    button { onclick: move |_| on_create_chat.call(()), "💬 New AI Chat" }
                }
            }
        }
    }
}

#[component]
fn InstanceItem(
    state: InstanceState,
    is_active: bool,
    on_select: EventHandler<String>,
    on_delete: EventHandler<String>,
) -> Element {
    let id = state.info.id.clone();
    let delete_id = id.clone();
    let icon = instance_icon(&state.info);
    let status_text = match state.info.status {
        InstanceStatus::Running => "● running",
        InstanceStatus::Stopped => "○ stopped",
        InstanceStatus::Error => "✕ error",
    };
    let active_class = if is_active { "active" } else { "" };
    rsx! {
        div {
            key: "{id}",
            class: "instance-item {active_class}",
            onclick: move |_| on_select.call(id.clone()),
            div { class: "instance-item-icon", "{icon}" }
            div { class: "instance-item-body",
                div { class: "instance-item-label", "{state.info.label}" }
                div { class: "instance-item-status", "{status_text}" }
            }
            button {
                class: "instance-item-close",
                title: "Kill and remove",
                onclick: move |e| {
                    e.stop_propagation();
                    on_delete.call(delete_id.clone());
                },
                "×"
            }
        }
    }
}

// -----------------------------------------------------------------------
// Instance panel

/// Main content panel for the selected instance.
#[component]
pub fn InstancePanel(
    instance: Option<InstanceState>,
    on_send: EventHandler<(String, String)>,
) -> Element {
    let Some(inst) = &instance else {
        return rsx! {
            div { class: "instance-panel empty-panel",
                div { class: "empty-msg", "Select an instance from the sidebar, or launch a new tool." }
            }
        };
    };

    let kind = inst.info.kind.clone();
    let id = inst.info.id.clone();
    let label = inst.info.label.clone();
    let term_lines = inst.term_lines.clone();
    let term_connected = inst.term_connected;
    let status = inst.info.status.clone();
    let messages = inst.messages.clone();

    match kind {
        InstanceKind::Terminal => rsx! {
            TerminalPanelView {
                label: label,
                term_lines: term_lines,
                term_connected: term_connected,
                running: status == InstanceStatus::Running,
                on_send: move |text| on_send.call((id.clone(), text)),
            }
        },
        InstanceKind::Chat => rsx! {
            ChatPanelView {
                label: label,
                messages: messages,
                on_send: move |text| on_send.call((id.clone(), text)),
            }
        },
    }
}

// -----------------------------------------------------------------------
// Terminal panel

#[component]
fn TerminalPanelView(
    label: String,
    term_lines: String,
    term_connected: bool,
    running: bool,
    on_send: EventHandler<String>,
) -> Element {
    let mut input = use_signal(String::new);

    rsx! {
        div { class: "term-instance",
            div { class: "term-header",
                span { class: "term-label", "{label}" }
                span {
                    class: if term_connected { "term-badge online" } else { "term-badge offline" },
                    if term_connected { "● connected" } else { "○ disconnected" }
                }
                if !running {
                    span { class: "term-badge done", "⏻ finished" }
                }
            }
            div { class: "term-output",
                pre { "{term_lines}" }
            }
            div { class: "term-input-row",
                input {
                    class: "term-input",
                    placeholder: "Type input and press Enter…",
                    value: "{input}",
                    disabled: !running,
                    oninput: move |e| input.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == dioxus::events::Key::Enter && !input.read().is_empty() {
                            let text = input.peek().clone();
                            input.set(String::new());
                            on_send.call(text);
                        }
                    },
                }
            }
        }
    }
}

// -----------------------------------------------------------------------
// Chat panel

#[component]
fn ChatPanelView(
    label: String,
    messages: Vec<ChatMsg>,
    on_send: EventHandler<String>,
) -> Element {
    let mut input = use_signal(String::new);

    rsx! {
        div { class: "chat-instance",
            div { class: "chat-header",
                span { class: "chat-label", "{label}" }
            }
            div { class: "chat-msgs",
                for (i, msg) in messages.iter().enumerate() {
                    ChatBubble { key: "{i}", role: msg.role.clone(), text: msg.text.clone() }
                }
            }
            div { class: "chat-input-row",
                input {
                    class: "chat-input",
                    placeholder: "Ask the AI…",
                    value: "{input}",
                    oninput: move |e| input.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == dioxus::events::Key::Enter && !input.read().is_empty() {
                            let text = input.peek().clone();
                            input.set(String::new());
                            on_send.call(text);
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn ChatBubble(role: crate::Role, text: String) -> Element {
    let class = match role {
        crate::Role::User => "msg-user",
        crate::Role::Ai => "msg-ai",
        crate::Role::Err => "msg-err",
    };
    rsx! {
        div { class: "chat-msg {class}",
            pre { "{text}" }
        }
    }
}

// -----------------------------------------------------------------------
// Helpers

/// Get a display icon for an instance based on its type or command.
fn instance_icon(info: &InstanceInfo) -> &'static str {
    match info.kind {
        InstanceKind::Chat => "💬",
        InstanceKind::Terminal => {
            match info.command.as_deref() {
                Some("claude") => "⚡",
                Some("codex") => "🧩",
                Some("opencode") => "📟",
                Some("gemini") => "✦",
                Some("aider") => "🛠",
                _ => ">_",
            }
        }
    }
}

/// Build a WebSocket URL for a terminal instance.
#[allow(unused)]
pub fn instance_ws_url(id: &str) -> String {
    let loc = web_sys::window().unwrap().location();
    let proto = if loc.protocol().unwrap_or_default() == "https:" { "wss" } else { "ws" };
    let host = loc.host().unwrap_or_else(|_| "127.0.0.1:8080".into());
    format!("{proto}://{host}/ws/pty/{id}")
}
