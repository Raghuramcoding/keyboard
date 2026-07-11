use dioxus::prelude::*;
use crate::state::AppState;
use crate::components::{source_control::SourceControl, scaffold_panel::ScaffoldPanel};
use tck_core::known_agents;

#[component]
pub fn Sidebar() -> Element {
    let state = use_context::<AppState>();
    let settings = state.settings.read().clone();
    let agents = known_agents();

    rsx! {
        div { class: "sidebar",
            div {
                div { class: "section-title", "AI Agents" }
                div { class: "btn-col",
                    for a in agents.iter() {
                        button {
                            key: "{a.id}",
                            onclick: {
                                let cmd = a.command.clone();
                                move |_| state.term.send(format!("{cmd}\r"))
                            },
                            "{a.icon}  {a.label}"
                        }
                    }
                }
            }
            div {
                div { class: "section-title", "Custom Commands" }
                div { class: "btn-col",
                    if settings.commands.is_empty() {
                        div { class: "muted", "Add commands in Settings." }
                    }
                    for c in settings.commands.iter() {
                        button {
                            key: "{c.id}",
                            onclick: {
                                let cmd = c.command.clone();
                                move |_| state.term.send(format!("{cmd}\r"))
                            },
                            "{c.icon}  {c.label}"
                        }
                    }
                }
            }
            SourceControl {}
            ScaffoldPanel {}
        }
    }
}