use dioxus::prelude::*;

#[component]
pub fn LandingPage() -> Element {
    rsx! {
        div { class: "tck-paper",
            LandingNav {}
            LandingHero {}
            PlatformsStrip {}
            FeaturesSection {}
            TornPaperDivider {}
            SimpleAppPreviewSection {}
            LandingFooter {}
        }
    }
}

#[component]
fn LandingNav() -> Element {
    rsx! {
        nav { class: "landing-nav landing-container",
            a { href: "/", class: "landing-brand",
                div { class: "landing-brand-mark",
                    svg { view_box: "0 0 26 17", width: "100%", height: "100%",
                        g { fill: "#fff8f1",
                            circle { cx: "8.5", cy: "9.5", r: "5.2" }
                            circle { cx: "15", cy: "7", r: "6.4" }
                            circle { cx: "19.5", cy: "10.5", r: "4.4" }
                            rect { x: "5", y: "9.5", width: "16", height: "5.5", rx: "2.7" }
                        }
                    }
                }
                div { class: "landing-brand-text",
                    div { class: "main", "T.C.K" }
                    div { class: "sub", "TalentCloud Keyboard" }
                }
            }
            div { class: "landing-nav-right",
                a { href: "https://github.com/Raghuramcoding/keyboard", target: "_blank", class: "landing-nav-link",
                    "GitHub"
                }
                a { href: "https://raghuramcoding.github.io/keyboard/#app", class: "tck-btn tck-btn--primary",
                    "Launch the app "
                    span { "→" }
                }
            }
        }
    }
}

#[component]
fn LandingHero() -> Element {
    rsx! {
        section { class: "landing-container",
            div { class: "hero",
                div { class: "hero-left",
                    div { class: "tck-eyebrow hero-left",
                        div { class: "eyebrow-rule" }
                        "AI CODING ENVIRONMENT · 100% RUST"
                    }
                    h1 { class: "hero-headline",
                        "Code with AI, "
                        span { class: "tck-mark", "entirely in Rust" }
                        ", right in your "
                        em { "browser." }
                    }
                    p { class: "hero-lead",
                        "T.C.K runs everything natively: a real shell, AI proxying, and a full editor—no cloud backend, no bloat. Pure Rust, compiled to WebAssembly and served from a lightweight axum server."
                    }
                    div { class: "hero-buttons",
                        a { href: "https://raghuramcoding.github.io/keyboard/#app", class: "tck-btn tck-btn--primary",
                            "Launch the app"
                        }
                        a { href: "https://github.com/Raghuramcoding/keyboard", target: "_blank", class: "tck-btn tck-btn--ghost",
                            "View on GitHub"
                        }
                    }
                    div { class: "hero-note",
                        "no install — it's all WebAssembly, promise."
                    }
                }
                div { class: "hero-right",
                    div { class: "field-note-card",
                        div { class: "field-note-tape" }
                        div { class: "field-note-code",
                            div { "$ trunk build --release" }
                            div { style: "margin-top: 8px; color: #8a8276;",
                                "Compiling tck-ui v0.1.0"
                            }
                            div { style: "margin-top: 8px;",
                                span { class: "success", "Finished" }
                                " release [optimized] target(s) in 12.4s"
                            }
                            div { style: "margin-top: 12px;",
                                "$ "
                                span { class: "tck-caret", "" }
                            }
                        }
                        div { class: "field-note-caption",
                            "↑ yes, the whole thing is Rust"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PlatformsStrip() -> Element {
    rsx! {
        div { class: "platforms-strip landing-container",
            div { class: "platforms-label", "RUNS ON" }
            div { class: "platforms-list",
                span { "macOS" }
                span { class: "sep", "·" }
                span { "Windows" }
                span { class: "sep", "·" }
                span { "Linux" }
                span { class: "sep", "·" }
                span { "iPhone" }
                span { class: "sep", "·" }
                span { "iPad" }
                span { class: "sep", "·" }
                span { "any modern browser" }
            }
        }
    }
}

#[component]
fn FeaturesSection() -> Element {
    rsx! {
        section { class: "features-section",
            div { class: "features-header landing-container",
                div {
                    div { class: "tck-eyebrow", "WHAT'S INSIDE" }
                }
                h2 { class: "features-title",
                    "Everything you need, in a single tab."
                }
                div { class: "features-aside",
                    "three little Rust crates doing all of this →"
                }
            }
            div { class: "landing-container",
                div { class: "features-grid",
                    FeatureCard {
                        icon: "⚙",
                        title: "100% Rust → WASM",
                        body: "Three crates compile to a WebAssembly UI and a native server. No JavaScript, no Electron—just Rust."
                    }
                    FeatureCard {
                        icon: "💻",
                        title: "A real terminal",
                        body: "A genuine shell—PowerShell or your $SHELL—streamed live to the browser over a WebSocket."
                    }
                    FeatureCard {
                        icon: "🤖",
                        title: "Multi-provider AI",
                        body: "Claude, Ollama, OpenRouter, and any OpenAI-compatible endpoint. Bring your own key, switch any time."
                    }
                    FeatureCard {
                        icon: "📦",
                        title: "Git via GitHub OAuth",
                        body: "Browse your repos, open files into the editor, and commit back. The token never leaves the server."
                    }
                    FeatureCard {
                        icon: "🎨",
                        title: "Project scaffolding",
                        body: "Generate Rust, web, Node, or Python starters—straight into the editor, or as a brand-new GitHub repo."
                    }
                    FeatureCard {
                        icon: "🚀",
                        title: "One-click agents",
                        body: "Fire up Claude Code, Codex, OpenCode, Gemini CLI, or Aider—right into the terminal, one button each."
                    }
                }
            }
        }
    }
}

#[component]
fn FeatureCard(icon: String, title: String, body: String) -> Element {
    rsx! {
        div { class: "feature-card",
            div { class: "feature-card-icon", "{icon}" }
            h3 { class: "feature-card-title", "{title}" }
            p { class: "feature-card-body", "{body}" }
        }
    }
}

#[component]
fn TornPaperDivider() -> Element {
    rsx! {
        div { class: "torn-paper",
            svg { view_box: "0 0 1160 60", width: "100%", height: "100%", preserve_aspect_ratio: "none",
                path {
                    d: "M0,20 Q290,15 580,20 T1160,20 L1160,60 L0,60 Z",
                    fill: "#0e1015"
                }
            }
        }
    }
}

#[component]
fn SimpleAppPreviewSection() -> Element {
    rsx! {
        section { class: "app-preview-section",
            div { class: "landing-container",
                div { class: "app-preview-header",
                    div { class: "app-preview-eyebrow", "THE REAL THING" }
                    h2 { class: "app-preview-title", "This is the actual app." }
                    p { class: "app-preview-subtitle", "Three columns: agents, editor, and AI chat." }
                }
                div { class: "app-cta",
                    div { class: "app-cta-buttons",
                        a { href: "https://raghuramcoding.github.io/keyboard/#app", class: "tck-btn tck-btn--aurora",
                            "Launch the app"
                        }
                        a { href: "https://github.com/Raghuramcoding/keyboard", target: "_blank", class: "tck-btn tck-btn--ghost",
                            style: "color: var(--fg); border-color: var(--fg);",
                            "View on GitHub"
                        }
                    }
                    p { class: "app-cta-caption", "free and open source mit" }
                }
            }
        }
    }
}

#[component]
fn LandingFooter() -> Element {
    rsx! {
        footer { class: "landing-footer",
            div { class: "footer-container",
                div { class: "footer-brand",
                    div { class: "footer-brand-mark",
                        svg { view_box: "0 0 26 17", width: "100%", height: "100%",
                            g { fill: "#fff8f1",
                                circle { cx: "8.5", cy: "9.5", r: "5.2" }
                                circle { cx: "15", cy: "7", r: "6.4" }
                                circle { cx: "19.5", cy: "10.5", r: "4.4" }
                                rect { x: "5", y: "9.5", width: "16", height: "5.5", rx: "2.7" }
                            }
                        }
                    }
                    div { class: "footer-brand-text",
                        div { class: "main", "T.C.K" }
                        div { class: "sub", "made entirely in Rust 🦀" }
                        img {
                            class: "footer-visitor-badge",
                            src: "https://visitor-badge.laobi.icu/badge?page_id=raghuramcoding.keyboard&left_color=%23262b3a&right_color=%235b8def",
                            alt: "unique visitor count",
                        }
                    }
                }
                div { class: "footer-links",
                    a { href: "https://github.com/Raghuramcoding/keyboard", target: "_blank", "GitHub" }
                    a { href: "https://raghuramcoding.github.io/keyboard/#app", "Launch app" }
                    a { href: "https://github.com/Raghuramcoding/keyboard/blob/main/LICENSE", target: "_blank", "MIT License" }
                }
                div { class: "footer-legal",
                    "Designed with intention."
                    br {}
                    "Built in pure Rust."
                    br {}
                    "MIT licensed."
                }
            }
        }
    }
}
