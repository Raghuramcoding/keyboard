//! Multi-instance dashboard — create, switch between, and dispatch to
//! multiple coding tool instances (terminal-based CLI tools + AI chat sessions).

use crate::state::{AppState, InstanceState, record_usage};
use crate::api::{fetch_instances, create_instance, write_stdin, delete_instance};
use crate::terminal::connect_instance_terminal;
use crate::{build_request, call_generate};
use dioxus::prelude::*;
use tck_core::{AgentState, ChatMsg, CreateInstanceRequest, InstanceInfo, InstanceKind, InstanceStatus, Role};

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
    let id_key = id.clone();
    let delete_id = id.clone();
    let icon = instance_icon(&state.info);
    let active_class = if is_active { "active" } else { "" };
    let state_dot_class = agent_state_css_class(state.agent_state);
    let state_label = agent_state_label(state.agent_state);
    rsx! {
        div {
            key: "{id_key}",
            class: "instance-item {active_class}",
            onclick: move |_| on_select.call(id.clone()),
            div { class: "instance-item-icon", "{icon}" }
            div { class: "instance-item-body",
                div { class: "instance-item-label", "{state.info.label}" }
                div { class: "instance-item-status",
                    span { class: "instance-state-dot {state_dot_class}" }
                    span { "{state_label}" }
                }
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
                agent_state: inst.agent_state,
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
    agent_state: AgentState,
    on_send: EventHandler<String>,
) -> Element {
    let mut input = use_signal(String::new);
    let badge_class = agent_state_css_class(agent_state);
    let badge_icon = agent_state_icon(agent_state);
    let badge_name = agent_state_label(agent_state);

    rsx! {
        div { class: "term-instance",
            div { class: "term-header",
                span { class: "term-label", "{label}" }
                span {
                    class: if term_connected { "term-badge online" } else { "term-badge offline" },
                    if term_connected { "● connected" } else { "○ disconnected" }
                }
                span { class: "agent-state-badge {badge_class}", "{badge_icon} {badge_name}" }
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

/// Map an AgentState to a CSS class name for the state dot.
fn agent_state_css_class(state: AgentState) -> &'static str {
    match state {
        AgentState::Idle => "idle",
        AgentState::Working => "working",
        AgentState::Done => "done",
        AgentState::Blocked => "blocked",
        AgentState::Error => "error",
    }
}

/// Human-readable label for the agent state.
fn agent_state_label(state: AgentState) -> &'static str {
    match state {
        AgentState::Idle => "idle",
        AgentState::Working => "processing",
        AgentState::Done => "completed",
        AgentState::Blocked => "waiting",
        AgentState::Error => "error",
    }
}

/// Icon character for the agent state badge (used in TerminalPanelView).
fn agent_state_icon(state: AgentState) -> &'static str {
    match state {
        AgentState::Idle => "○",
        AgentState::Working => "◉",
        AgentState::Done => "✓",
        AgentState::Blocked => "◷",
        AgentState::Error => "✕",
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

/// The main dashboard component that combines the sidebar and instance panel.
#[component]
pub fn InstanceDashboard() -> Element {
    let mut state = use_context::<AppState>();

    use_future(move || async move {
        let list = fetch_instances().await;
        let mut instances = state.instances;
        let instates: Vec<InstanceState> = list
            .into_iter()
            .map(|info| {
                let agent_state = info.agent_state;
                InstanceState {
                    info,
                    term_lines: String::new(),
                    term_connected: false,
                    messages: Vec::new(),
                    agent_state,
                }
            })
            .collect();
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
                        let req = CreateInstanceRequest {
                            kind: InstanceKind::Terminal,
                            label: Some(cmd.clone()),
                            command: Some(cmd.clone()),
                            cwd: None,
                        };
                        if let Some(info) = create_instance(&req).await {
                            let mut list = state.instances.peek().clone();
                            let agent_state = info.agent_state;
                            list.push(InstanceState {
                                info: info.clone(),
                                term_lines: String::new(),
                                term_connected: false,
                                messages: Vec::new(),
                                agent_state,
                            });
                            state.instances.set(list);
                            let id = info.id.clone();
                            state.active_id.set(Some(id.clone()));
                            // Connect terminal after creation
                            connect_instance_terminal(state, id);
                        }
                    });
                },
                on_create_chat: move |_| {
                    let id = format!("chat_{:016x}", rand_u64());
                    let info = InstanceInfo {
                        id: id.clone(),
                        kind: InstanceKind::Chat,
                        label: "AI Chat".into(),
                        command: None,
                        cwd: None,
                        status: InstanceStatus::Running,
                        error: None,
                        created_at: js_sys::Date::now() as u64,
                        agent_state: AgentState::default(),
                        last_output_at: 0,
                    };
                    let mut list = state.instances.peek().clone();
                    list.push(InstanceState {
                        info,
                        term_lines: String::new(),
                        term_connected: false,
                        messages: Vec::new(),
                        agent_state: AgentState::default(),
                    });
                    state.instances.set(list);
                    state.active_id.set(Some(id));
                },
                on_delete: move |id: String| {
                    let id_c = id.clone();
                    spawn(async move {
                        delete_instance(&id_c).await;
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

fn send_to_instance(mut state: AppState, id: &str, text: &str) {
    let id_str = id.to_string();
    let text_str = text.to_string();
    spawn(async move {
        write_stdin(&id_str, &text_str).await;

        let mut list = state.instances.peek().clone();
        let kind = list.iter().find(|s| s.info.id == id_str).map(|s| s.info.kind.clone());

        if kind == Some(InstanceKind::Terminal) {
            if let Some(inst) = list.iter_mut().find(|s| s.info.id == id_str) {
                inst.term_lines.push_str(&format!("$ {text_str}\n"));
            }
            state.instances.set(list);
            return;
        }

        if let Some(inst) = list.iter_mut().find(|s| s.info.id == id_str) {
            inst.messages.push(ChatMsg {
                role: Role::User,
                text: text_str.clone(),
            });
        }
        state.instances.set(list);

        let req = build_request(&state, text_str, Some("You are T.C.K, a helpful AI coding assistant.".into()));
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