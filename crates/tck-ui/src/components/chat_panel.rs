use dioxus::prelude::*;
use crate::state::{AppState, record_usage};
use crate::{build_request, call_generate};
use tck_core::{Role, ChatMsg};

#[component]
pub fn ChatPanel() -> Element {
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
        let req = build_request(&state, text, Some("You are T.C.K, a helpful AI coding assistant.".into()));
        let has_server = *state.server.peek();
        let provider_id = state.provider.peek().clone();
        crate::spawn(async move {
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
                for (i, m) in messages.iter().enumerate() {
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