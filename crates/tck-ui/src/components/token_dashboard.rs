use dioxus::prelude::*;
use tck_core::{AgentTokenUsage, agent_color, usage_pct};

/// Vibrant donut chart + agent-usage table showing which AI tools consume the most
/// tokens across the whole session. Always visible at the top of the right panel.
#[component]
pub fn TokenUsageDashboard(token_usage: Signal<Vec<AgentTokenUsage>>) -> Element {
    let agents = token_usage.read().clone();
    let total_chars: usize = agents.iter().map(|a| a.chars_produced).sum();
    let total_tokens: usize = total_chars / 4;

    let mut collapsed = use_signal(|| true);

    // Pre-compute legend rows so we don't mix complex Rust expressions inside rsx.
    let legend_rows: Vec<LegendRow> = agents
        .iter()
        .map(|a| {
            let pct_str = usage_pct(a.chars_produced, total_chars);
            let pct_num = pct_str.trim_end_matches('%').to_string();
            let color = agent_color(&a.agent_id).to_string();
            LegendRow {
                icon: a.icon.clone(),
                name: a.name.clone(),
                tokens: a.tokens,
                color,
                pct_str,
                pct_num,
                bar_width: if total_chars > 0 {
                    100.0 * a.chars_produced as f64 / total_chars as f64
                } else {
                    0.0
                },
            }
        })
        .collect();

    rsx! {
        div { class: "tkd",
            div { class: "tkd-header",
                div { class: "tkd-header-left",
                    span { class: "tkd-icon", "⚡" }
                    div {
                        div { class: "tkd-title", "Token Dashboard" }
                        div { class: "tkd-sub", "{total_tokens} tot · {agents.len()} tools" }
                    }
                }
                button {
                    class: "tkd-toggle",
                    onclick: move |_| {
                        let cur = *collapsed.peek();
                        collapsed.set(!cur);
                    },
                    if *collapsed.read() { "▸" } else { "▾" }
                }
            }
            if !*collapsed.read() && !agents.is_empty() {
                div { class: "tkd-body",
                    div { class: "tkd-donut-wrap",
                        DonutChart { agents: agents.clone(), total_chars }
                    }
                    div { class: "tkd-legend",
                        for row in legend_rows.iter() {
                            div { class: "tkd-legend-item",
                                span {
                                    class: "tkd-legend-dot",
                                    style: format!("background:{};", row.color),
                                }
                                span { class: "tkd-legend-name",
                                    "{row.icon} {row.name}"
                                }
                                span { class: "tkd-legend-bar-wrap",
                                    span {
                                        class: "tkd-legend-bar",
                                        style: format!("width:{}%;background:{};", row.pct_num, row.color),
                                    }
                                }
                                span { class: "tkd-legend-pct", "{row.pct_str}" }
                                span { class: "tkd-legend-abs", "{row.tokens} tok" }
                            }
                        }
                    }
                }
            }
            if *collapsed.read() && !agents.is_empty() {
                div { class: "tkd-bar-row",
                        for row in legend_rows.iter() {
                            div {
                                class: "tkd-bar-mini",
                                title: format!("{}: {} tokens ({})", row.name, row.tokens, row.pct_str),
                                style: format!("flex:{};background:{};", row.bar_width.max(1.0) as usize, row.color),
                            }
                        }
                }
            }
        }
    }
}

#[derive(Clone)]
struct LegendRow {
    icon: String,
    name: String,
    tokens: usize,
    color: String,
    pct_str: String,
    pct_num: String,
    bar_width: f64,
}

/// SVG donut chart — one arc per tracked agent, sized by token consumption.
#[component]
fn DonutChart(agents: Vec<AgentTokenUsage>, total_chars: usize) -> Element {
    let radius = 66.0;
    let stroke_w = 26.0;
    let circumference = 2.0 * std::f64::consts::PI * radius;

    // Pre-compute all arcs before entering rsx! (Dioxus for-body can't execute
    // arbitrary statements between elements).
    struct ArcSeg {
        key: String,
        color: String,
        dasharray: String,
        dashoffset: String,
        tooltip: String,
    }
    let gap = 2.0;
    let mut cumulative = 0.0;
    let mut arcs: Vec<ArcSeg> = agents
        .iter()
        .map(|a| {
            let p = if total_chars > 0 {
                a.chars_produced as f64 / total_chars as f64
            } else {
                0.0
            };
            let seg_len = circumference * p;
            let draw = (seg_len - gap).max(0.0);
            let rest = circumference - draw;
            let offset = cumulative;
            cumulative += seg_len;
            let pct_str = usage_pct(a.chars_produced, total_chars);
            ArcSeg {
                key: a.agent_id.clone(),
                color: agent_color(&a.agent_id).to_string(),
                dasharray: format!("{draw:.1} {rest:.1}"),
                dashoffset: format!("{:.1}", -offset),
                tooltip: format!("{}: {} tokens ({})", a.name, a.tokens, pct_str),
            }
        })
        .collect();
    // Largest arc first for best visual layering.
    arcs.sort_by(|a, b| {
        b.dasharray
            .len()
            .partial_cmp(&a.dasharray.len())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    rsx! {
        div { class: "tkd-donut",
            svg { view_box: "0 0 200 200",
                circle { cx: "100", cy: "100", r: "{radius}",
                    fill: "none", stroke: "#2d2d30", "stroke-width": "{stroke_w}" }
                for s in arcs.iter() {
                    circle {
                        key: "{s.key}",
                        cx: "100", cy: "100", r: "{radius}",
                        fill: "none",
                        stroke: "{s.color}",
                        "stroke-width": "{stroke_w}",
                        "stroke-dasharray": "{s.dasharray}",
                        "stroke-dashoffset": "{s.dashoffset}",
                        "stroke-linecap": "round",
                        transform: "rotate(-90 100 100)",
                        title { "{s.tooltip}" }
                    }
                }
                text {
                    x: "100", y: "92",
                    text_anchor: "middle",
                    fill: "#d4d4d4",
                    "font-size": "18",
                    "font-weight": "700",
                    "font-family": "system-ui, sans-serif",
                    "{format_tk(total_chars / 4)}"
                }
                text {
                    x: "100", y: "112",
                    text_anchor: "middle",
                    fill: "#858585",
                    "font-size": "11",
                    "font-family": "system-ui, sans-serif",
                    "tokens across {agents.len()} agents"
                }
            }
        }
    }
}

/// Format a number with K/M suffix for the center of the donut.
fn format_tk(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}