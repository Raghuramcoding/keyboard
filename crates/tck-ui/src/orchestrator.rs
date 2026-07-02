//! Multi-agent orchestrator: a planner agent decomposes a goal into subtasks,
//! specialist worker agents run them concurrently, and a synthesizer merges the
//! results. Everything runs through `call_generate`, so it works both against a
//! native T.C.K server and fully in-browser on a static host.

use crate::{build_request_sys, call_generate, record_usage, AppState};
use dioxus::prelude::*;
use futures_util::future::join_all;
use tck_core::{
    agent_color, fallback_plan, orchestrator_roles, parse_plan, planner_prompt, planner_system,
    role_by_id, synthesizer_prompt, synthesizer_system, worker_prompt, PlannedTask,
};

#[derive(Clone, Copy, PartialEq)]
pub enum Phase {
    Idle,
    Planning,
    Working,
    Synthesizing,
    Done,
    Failed,
}

#[derive(Clone, Copy, PartialEq)]
enum TaskStatus {
    Running,
    Done,
    Failed,
}

#[derive(Clone, PartialEq)]
struct TaskCard {
    task: PlannedTask,
    status: TaskStatus,
    output: String,
}

#[component]
pub fn OrchestratorPanel() -> Element {
    let state = use_context::<AppState>();
    let mut goal = use_signal(String::new);
    let cards = use_signal(Vec::<TaskCard>::new);
    let phase = use_signal(|| Phase::Idle);
    let planner_out = use_signal(String::new);
    let final_out = use_signal(String::new);
    let error = use_signal(String::new);

    let ph = *phase.read();
    let running = matches!(ph, Phase::Planning | Phase::Working | Phase::Synthesizing);
    let cards_now = cards.read().clone();
    let err_now = error.read().clone();
    let final_now = final_out.read().clone();

    let run = move |_| {
        let g = goal.peek().trim().to_string();
        if g.is_empty() || running {
            return;
        }
        run_orchestration(state, g, cards, phase, planner_out, final_out, error);
    };

    rsx! {
        div { class: "orch",
            div { class: "orch-in",
                textarea {
                    placeholder: "Describe a goal for the agent team…",
                    value: "{goal}",
                    oninput: move |e| goal.set(e.value()),
                }
                button { class: "primary", disabled: running, onclick: run,
                    if running { "⏳ Orchestrating…" } else { "🎭 Run agent team" }
                }
            }
            AgentActivity { cards: cards_now.clone(), phase: ph, planner_out: planner_out.read().clone(), final_out: final_now.clone() }
            if ph == Phase::Planning {
                div { class: "orch-phase", "🧭 Planner is decomposing the goal…" }
            }
            div { class: "orch-cards",
                for (i , c) in cards_now.iter().enumerate() {
                    div {
                        key: "{i}",
                        class: "orch-card",
                        style: "border-left: 3px solid {agent_color(&c.task.role)};",
                        div { class: "orch-card-head",
                            span { class: "orch-role", "{role_by_id(&c.task.role).icon} {role_by_id(&c.task.role).name}" }
                            span {
                                class: match c.status {
                                    TaskStatus::Running => "orch-status run",
                                    TaskStatus::Done => "orch-status ok",
                                    TaskStatus::Failed => "orch-status bad",
                                },
                                match c.status {
                                    TaskStatus::Running => "working…",
                                    TaskStatus::Done => "done",
                                    TaskStatus::Failed => "failed",
                                }
                            }
                        }
                        div { class: "orch-title", "{c.task.title}" }
                        if !c.output.is_empty() {
                            details {
                                summary { "output ({c.output.len()} chars)" }
                                pre { class: "orch-out", "{c.output}" }
                            }
                        }
                    }
                }
            }
            if ph == Phase::Synthesizing {
                div { class: "orch-phase", "🧵 Synthesizer is merging results…" }
            }
            if !final_now.is_empty() {
                div { class: "orch-final",
                    div { class: "orch-final-head",
                        span { "🏁 Final result" }
                        button {
                            class: "linkish",
                            onclick: move |_| {
                                let mut code = state.code;
                                let mut filename = state.filename;
                                code.set(final_out.peek().clone());
                                filename.set("orchestration.md".to_string());
                            },
                            "open in editor"
                        }
                    }
                    pre { class: "orch-out", "{final_now}" }
                }
            }
            if !err_now.is_empty() {
                div { class: "orch-err", "{err_now}" }
            }
        }
    }
}

/// "Who is working the most on what": one sphere per active agent, sized by how
/// much output that agent has produced so far, colored by role, pulsing while busy.
#[component]
fn AgentActivity(
    cards: Vec<TaskCard>,
    phase: Phase,
    planner_out: String,
    final_out: String,
) -> Element {
    // (role id, name, icon, chars produced, busy, what it's working on)
    let mut agents: Vec<(String, String, String, usize, bool, String)> = Vec::new();
    agents.push((
        "planner".into(),
        "Planner".into(),
        "🧭".into(),
        planner_out.len().max(1),
        phase == Phase::Planning,
        "decompose the goal".into(),
    ));
    for role in orchestrator_roles() {
        let mine: Vec<&TaskCard> = cards.iter().filter(|c| c.task.role == role.id).collect();
        if mine.is_empty() {
            continue;
        }
        let chars: usize = mine.iter().map(|c| c.output.len()).sum();
        let busy = mine.iter().any(|c| c.status == TaskStatus::Running);
        let titles: Vec<String> = mine.iter().map(|c| c.task.title.clone()).collect();
        agents.push((role.id, role.name, role.icon, chars.max(1), busy, titles.join(" · ")));
    }
    // Unknown roles the planner may have invented still get a sphere.
    for c in &cards {
        if !agents.iter().any(|a| a.0 == c.task.role) {
            let r = role_by_id(&c.task.role);
            agents.push((
                r.id,
                r.name,
                r.icon,
                c.output.len().max(1),
                c.status == TaskStatus::Running,
                c.task.title.clone(),
            ));
        }
    }
    agents.push((
        "synthesizer".into(),
        "Synthesizer".into(),
        "🧵".into(),
        final_out.len().max(1),
        phase == Phase::Synthesizing,
        "merge all results".into(),
    ));

    if phase == Phase::Idle && cards.is_empty() {
        return rsx! {
            div { class: "orch-viz-empty",
                "Agent activity will appear here — bigger sphere = busier agent."
            }
        };
    }

    let max_chars = agents.iter().map(|a| a.3).max().unwrap_or(1) as f64;
    let total_chars: usize = agents.iter().map(|a| a.3).sum();

    rsx! {
        div { class: "orch-viz",
            for (id , name , icon , chars , busy , what) in agents.into_iter() {
                div { key: "{id}", class: "orch-agent", title: "{name}: {what}",
                    div {
                        class: if busy { "orch-sphere busy" } else { "orch-sphere" },
                        style: {
                            let size = 26.0 + 44.0 * (chars as f64 / max_chars);
                            let color = agent_color(&id);
                            format!(
                                "width:{size:.0}px;height:{size:.0}px;background:radial-gradient(circle at 32% 28%, #ffffff88, {color} 55%, #000000cc 130%);box-shadow:0 0 {glow:.0}px {color};",
                                glow = if busy { 14.0 } else { 4.0 },
                            )
                        },
                        span { class: "orch-sphere-icon", "{icon}" }
                    }
                    div { class: "orch-agent-name", "{name}" }
                    div { class: "orch-agent-share",
                        {format!("{:.0}%", 100.0 * chars as f64 / total_chars.max(1) as f64)}
                    }
                    div { class: "orch-agent-what", "{what}" }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_orchestration(
    state: AppState,
    goal: String,
    mut cards: Signal<Vec<TaskCard>>,
    mut phase: Signal<Phase>,
    mut planner_out: Signal<String>,
    mut final_out: Signal<String>,
    mut error: Signal<String>,
) {
    let has_server = *state.server.peek();
    cards.set(Vec::new());
    final_out.set(String::new());
    planner_out.set(String::new());
    error.set(String::new());
    phase.set(Phase::Planning);

    spawn(async move {
        // ---- 1. Plan ----
        let req = build_request_sys(&state, planner_prompt(&goal), planner_system());
        let resp = call_generate(req, has_server).await;
        let plan = match (resp.response, resp.error) {
            (Some(text), _) => {
                record_usage(state.token_usage, "planner", text.len());
                planner_out.set(text.clone());
                let parsed = parse_plan(&text);
                if parsed.is_empty() {
                    fallback_plan(&goal)
                } else {
                    parsed
                }
            }
            (_, err) => {
                error.set(format!(
                    "Planner failed: {}",
                    err.unwrap_or_else(|| "empty response".into())
                ));
                phase.set(Phase::Failed);
                return;
            }
        };

        cards.set(
            plan.iter()
                .map(|t| TaskCard {
                    task: t.clone(),
                    status: TaskStatus::Running,
                    output: String::new(),
                })
                .collect(),
        );
        phase.set(Phase::Working);

        // ---- 2. Workers (concurrent) ----
        let cards_w = cards;
        let usage_w = state.token_usage;
        let workers = plan.iter().enumerate().map(|(i, task)| {
            let task = task.clone();
            let goal = goal.clone();
            let mut cards = cards_w;
            let token_usage = usage_w;
            async move {
                let role = role_by_id(&task.role);
                let req = build_request_sys(&state, worker_prompt(&goal, &task), role.system_prompt);
                let resp = call_generate(req, has_server).await;
                let (status, output) = match (resp.response, resp.error) {
                    (Some(text), _) => {
                        record_usage(token_usage, &task.role, text.len());
                        (TaskStatus::Done, text)
                    }
                    (_, err) => (
                        TaskStatus::Failed,
                        format!("Error: {}", err.unwrap_or_else(|| "empty response".into())),
                    ),
                };
                let mut list = cards.peek().clone();
                if let Some(c) = list.get_mut(i) {
                    c.status = status;
                    c.output = output.clone();
                }
                cards.set(list);
                (status, output)
            }
        });
        let results: Vec<(TaskStatus, String)> = join_all(workers).await;

        let ok: Vec<(PlannedTask, String)> = plan
            .iter()
            .cloned()
            .zip(results.iter().cloned())
            .filter(|(_, (s, _))| *s == TaskStatus::Done)
            .map(|(t, (_, out))| (t, out))
            .collect();
        if ok.is_empty() {
            error.set("All worker agents failed — check your provider/model settings.".into());
            phase.set(Phase::Failed);
            return;
        }

        // ---- 3. Synthesize ----
        phase.set(Phase::Synthesizing);
        let req = build_request_sys(&state, synthesizer_prompt(&goal, &ok), synthesizer_system());
        let resp = call_generate(req, has_server).await;
        match (resp.response, resp.error) {
            (Some(text), _) => {
                record_usage(state.token_usage, "synthesizer", text.len());
                final_out.set(text);
                phase.set(Phase::Done);
            }
            (_, err) => {
                error.set(format!(
                    "Synthesizer failed: {}",
                    err.unwrap_or_else(|| "empty response".into())
                ));
                phase.set(Phase::Failed);
            }
        }
    });
}
