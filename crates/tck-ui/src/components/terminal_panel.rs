use dioxus::prelude::*;
use crate::state::AppState;

#[component]
pub fn TerminalPanel() -> Element {
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
                        if e.key() == dioxus::events::Key::Enter {
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