use dioxus::prelude::*;
use crate::state::AppState;
use crate::components::{chat_panel::ChatPanel, token_dashboard::TokenUsageDashboard};
use crate::orchestrator::OrchestratorPanel;

/// The right-hand panel hosts two tools: the single-model AI Chat and the
/// multi-agent Orchestrator. Both stay mounted (state survives tab switches);
/// the inactive one is hidden with CSS.
#[derive(Clone, Copy, PartialEq)]
enum RightTab {
    Chat,
    Orchestrate,
}

#[component]
pub fn RightPanel() -> Element {
    let state = use_context::<AppState>();
    let mut tab = use_signal(|| RightTab::Orchestrate);
    let t = *tab.read();
    rsx! {
        div { class: "chat",
            TokenUsageDashboard { token_usage: state.token_usage }
            div { class: "right-tabs",
                button {
                    class: if t == RightTab::Chat { "rtab active" } else { "rtab" },
                    onclick: move |_| tab.set(RightTab::Chat),
                    "🤖 AI Chat"
                }
                button {
                    class: if t == RightTab::Orchestrate { "rtab active" } else { "rtab" },
                    onclick: move |_| tab.set(RightTab::Orchestrate),
                    "🎭 Orchestrator"
                }
            }
            div { class: if t == RightTab::Chat { "right-body" } else { "right-body hidden" },
                ChatPanel {}
            }
            div { class: if t == RightTab::Orchestrate { "right-body" } else { "right-body hidden" },
                OrchestratorPanel {}
            }
        }
    }
}