//! T.C.K — TalentCloud Keyboard
//! A pure-Rust (Dioxus/WASM) AI coding environment that runs in the browser.

mod api;
mod components;
mod landing;
mod orchestrator;
mod state;
mod terminal;

use dioxus::prelude::*;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::{SinkExt, StreamExt};
use gloo_net::http::Request;
use gloo_net::websocket::{futures::WebSocket, Message as WsMessage};
use landing::LandingPage;
use serde_json::{json, Value};
use tck_core::{
    EventMessage, GenerateRequest, GenerateResponse, Role,
};
use wasm_bindgen::JsCast;

use crate::components::{
    title_bar::TitleBar,
    sidebar::Sidebar,
    right_panel::RightPanel,
    settings_modal::SettingsModal,
    instance_dashboard::InstanceDashboard,
};
use crate::state::{AppState, InstanceState};
use crate::terminal::ws_url;

fn main() {
    dioxus::launch(App);
}

fn check_show_landing() -> bool {
    web_sys::window()
        .map(|w| check_show_landing_in(&w))
        .unwrap_or(true)
}

fn check_show_landing_in(window: &web_sys::Window) -> bool {
    match window.location().hash() {
        Ok(hash) => hash.is_empty() || hash == "#",
        Err(_) => true,
    }
}

#[component]
fn App() -> Element {
    let mut show_landing = use_signal(check_show_landing);

    use_effect(move || {
        if let Some(window) = web_sys::window() {
            let win = window.clone();
            let closure = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
                show_landing.set(check_show_landing_in(&win));
            });
            let _ = window.add_event_listener_with_callback("hashchange", closure.as_ref().unchecked_ref());
            closure.forget();
        }
    });

    if *show_landing.read() {
        return rsx! { LandingPage {} };
    }

    rsx! { AppMain {} }
}

#[component]
fn AppMain() -> Element {
    // Terminal coroutine
    let term = use_coroutine(move |mut rx: UnboundedReceiver<String>| async move {
        match WebSocket::open(&ws_url()) {
            Ok(ws) => {
                let (mut write, mut read) = ws.split();
                spawn(async move {
                    let mut received = false;
                    while let Some(item) = read.next().await {
                        let msg = match item {
                            Ok(m) => m,
                            Err(_) => break,
                        };
                        if !received {
                            received = true;
                            // term_connected is set via state context
                        }
                        let _text = match msg {
                            WsMessage::Text(t) => t,
                            WsMessage::Bytes(b) => String::from_utf8_lossy(&b).to_string(),
                        };
                        // term_lines is set via state context
                    }
                    // term_connected is set via state context
                    if !received {
                        // term_lines is set via state context
                    }
                });
                while let Some(input) = rx.next().await {
                    if write.send(WsMessage::Text(input)).await.is_err() {
                        break;
                    }
                }
            }
            Err(_) => {
                // term_lines and term_connected are set via state context
            }
        }
    });

    // AppState is now created in state.rs with all signals
    let state = use_context_provider(move || AppState::new(term));

    // Subscribe to server events
    subscribe_events(state.instances, state.server);

    rsx! {
        div { class: "app",
            TitleBar {}
            div { class: "main-layout",
                Sidebar {}
                div { class: "center-panel",
                    InstanceDashboard {}
                }
                RightPanel {}
            }
            if *state.show_settings.read() {
                SettingsModal {}
            }
        }
    }
}

async fn subscribe_events(mut instances: Signal<Vec<InstanceState>>, server: Signal<bool>) {
    let events_ws_url = {
        let loc = web_sys::window().unwrap().location();
        let proto = if loc.protocol().unwrap_or_default() == "https:" { "wss" } else { "ws" };
        let host = loc.host().unwrap_or_else(|_| "127.0.0.1:8080".into());
        format!("{proto}://{host}/ws/events")
    };

    spawn(async move {
        loop {
            if !*server.peek() {
                gloo_timers::future::TimeoutFuture::new(1000).await;
                continue;
            }
            match WebSocket::open(&events_ws_url) {
                Ok(ws) => {
                    let (_write, mut read) = ws.split();
                    while let Some(item) = read.next().await {
                        match item {
                            Ok(WsMessage::Text(text)) => {
                                if let Ok(event) = serde_json::from_str::<EventMessage>(&text) {
                                    handle_event(event, &mut instances);
                                }
                            }
                            Ok(WsMessage::Bytes(_)) => {}
                            Err(_) => break,
                        }
                    }
                }
                Err(_) => {}
            }
            gloo_timers::future::TimeoutFuture::new(3000).await;
        }
    });
}

fn handle_event(event: EventMessage, instances: &mut Signal<Vec<InstanceState>>) {
    match event {
        EventMessage::AgentStateChanged { instance_id, new_state, .. } => {
            let mut list = instances.peek().clone();
            if let Some(inst) = list.iter_mut().find(|s| s.info.id == instance_id) {
                inst.agent_state = new_state;
                instances.set(list);
            }
        }
        EventMessage::InstanceStatusChanged { instance_id, status, .. } => {
            let mut list = instances.peek().clone();
            if let Some(inst) = list.iter_mut().find(|s| s.info.id == instance_id) {
                inst.info.status = status;
                instances.set(list);
            }
        }
        _ => {}
    }
}

pub fn build_request(state: &AppState, prompt: String, system: Option<String>) -> GenerateRequest {
    let settings = state.settings.peek().clone();
    let provider = state.provider.peek().clone();
    let model = state.model.peek().clone();
    let mut req = GenerateRequest {
        provider: provider.clone(),
        model,
        prompt,
        system: system.or_else(|| Some("You are T.C.K, a helpful AI coding assistant.".into())),
        base_url: None,
        api_key: None,
        temperature: None,
        max_tokens: None,
    };
    match provider.as_str() {
        "ollama" => {
            if !settings.ollama_host.is_empty() {
                req.base_url = Some(settings.ollama_host);
            }
        }
        "claude" => {
            req.api_key = Some(settings.claude_api_key);
        }
        id => {
            if let Some(p) = settings.providers.iter().find(|p| p.id == id) {
                req.base_url = Some(p.base_url.clone());
                req.api_key = Some(p.api_key.clone());
            }
        }
    }
    req
}

async fn call_generate(req: GenerateRequest, has_server: bool) -> GenerateResponse {
    if !has_server {
        return generate_direct(req).await;
    }
    let built = match Request::post("/api/ai/generate").json(&req) {
        Ok(b) => b,
        Err(e) => {
            return GenerateResponse {
                error: Some(format!("Request build failed: {e}")),
                ..Default::default()
            }
        }
    };
    match built.send().await {
        Ok(resp) => resp
            .json::<GenerateResponse>()
            .await
            .unwrap_or_else(|e| GenerateResponse {
                error: Some(format!("Bad response from server: {e}")),
                ..Default::default()
            }),
        Err(e) => GenerateResponse {
            error: Some(format!(
                "Cannot reach the T.C.K server: {e}. Start it with `cargo run -p tck-server`."
            )),
            ..Default::default()
        },
    }
}

use std::sync::OnceLock;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

struct TckAIBridge {
    namespace: JsValue,
    generate_fn: JsValue,
}

static TCKAI: OnceLock<Option<TckAIBridge>> = OnceLock::new();

fn init_tck_ai() -> Option<&'static TckAIBridge> {
    TCKAI.get_or_init(|| {
        let win = web_sys::window()?;
        let ns = js_sys::Reflect::get(&win, &JsValue::from_str("TckAI")).ok()?;
        if ns.is_undefined() || ns.is_null() { return None; }
        let gen = js_sys::Reflect::get(&ns, &JsValue::from_str("generate")).ok()?;
        Some(TckAIBridge { namespace: ns, generate_fn: gen })
    }).as_ref()
}

async fn generate_via_bridge(req: &GenerateRequest) -> Option<GenerateResponse> {
    let bridge = init_tck_ai()?;

    let js_req = js_sys::Object::new();
    js_sys::Reflect::set(&js_req, &JsValue::from_str("provider"), &JsValue::from_str(&req.provider)).ok()?;
    js_sys::Reflect::set(&js_req, &JsValue::from_str("model"), &JsValue::from_str(&req.model)).ok()?;
    js_sys::Reflect::set(&js_req, &JsValue::from_str("prompt"), &JsValue::from_str(&req.prompt)).ok()?;
    if let Some(sys) = &req.system {
        js_sys::Reflect::set(&js_req, &JsValue::from_str("system"), &JsValue::from_str(sys)).ok()?;
    }
    if let Some(k) = &req.api_key {
        js_sys::Reflect::set(&js_req, &JsValue::from_str("apiKey"), &JsValue::from_str(k)).ok()?;
    }
    if let Some(b) = &req.base_url {
        js_sys::Reflect::set(&js_req, &JsValue::from_str("baseUrl"), &JsValue::from_str(b)).ok()?;
    }
    if let Some(t) = req.temperature {
        js_sys::Reflect::set(&js_req, &JsValue::from_str("temperature"), &JsValue::from_f64(t as f64)).ok()?;
    }
    if let Some(m) = req.max_tokens {
        js_sys::Reflect::set(&js_req, &JsValue::from_str("maxTokens"), &JsValue::from_f64(m as f64)).ok()?;
    }

    let gen_fn = bridge.generate_fn.dyn_ref::<js_sys::Function>()?;
    let promise = gen_fn.call1(&bridge.namespace, &js_req).ok()?;
    let promise = promise.dyn_into::<js_sys::Promise>().ok()?;
    let result = JsFuture::from(promise).await.ok()?;

    let response = js_sys::Reflect::get(&result, &JsValue::from_str("response"))
        .ok()?.as_string().unwrap_or_default();
    let model = js_sys::Reflect::get(&result, &JsValue::from_str("model"))
        .ok()?.as_string().unwrap_or_default();

    Some(GenerateResponse {
        response: Some(response),
        model: Some(model),
        ..Default::default()
    })
}

async fn generate_direct(req: GenerateRequest) -> GenerateResponse {
    if let Some(resp) = generate_via_bridge(&req).await {
        return resp;
    }
    let result = match req.provider.as_str() {
        "ollama" => direct_ollama(&req).await,
        "claude" => direct_claude(&req).await,
        _ => direct_openai(&req).await,
    };
    match result {
        Ok(text) => GenerateResponse {
            response: Some(text),
            model: Some(req.model),
            ..Default::default()
        },
        Err(e) => GenerateResponse {
            error: Some(e),
            ..Default::default()
        },
    }
}

async fn direct_ollama(req: &GenerateRequest) -> Result<String, String> {
    let host = req
        .base_url
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://localhost:11434".to_string());
    let url = format!("{}/api/generate", host.trim_end_matches('/'));
    let body = json!({
        "model": req.model, "prompt": req.prompt,
        "system": req.system.clone().unwrap_or_default(), "stream": false,
    });
    let resp = Request::post(&url)
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| format!("Cannot reach Ollama: {e}"))?;
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Bad Ollama response: {e}"))?;
    if let Some(err) = v.get("error") {
        return Err(format!(
            "Ollama error: {}",
            err.as_str().unwrap_or("unknown")
        ));
    }
    Ok(v["response"].as_str().unwrap_or_default().to_string())
}

async fn direct_claude(req: &GenerateRequest) -> Result<String, String> {
    let key = req.api_key.clone().unwrap_or_default();
    if key.is_empty() {
        return Err("No Claude API key configured (set it in Settings).".to_string());
    }
    let body = json!({
        "model": req.model, "max_tokens": req.max_tokens.unwrap_or(2048),
        "system": req.system.clone().unwrap_or_default(),
        "messages": [{ "role": "user", "content": req.prompt }],
    });
    let resp = Request::post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-dangerous-direct-browser-access", "true")
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| format!("Cannot reach Claude API: {e}"))?;
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Bad Claude response: {e}"))?;
    if let Some(err) = v.get("error") {
        return Err(format!("Claude error: {err}"));
    }
    Ok(v["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string())
}

async fn direct_openai(req: &GenerateRequest) -> Result<String, String> {
    let base = req.base_url.clone().unwrap_or_default();
    if base.is_empty() {
        return Err(format!(
            "Provider \"{}\" has no base URL configured.",
            req.provider
        ));
    }
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));
    let mut messages = Vec::new();
    if let Some(sys) = &req.system {
        if !sys.is_empty() {
            messages.push(json!({ "role": "system", "content": sys }));
        }
    }
    messages.push(json!({ "role": "user", "content": req.prompt }));
    let mut body = json!({ "model": req.model, "messages": messages });
    if let Some(t) = req.temperature {
        body["temperature"] = json!(t);
    }
    if let Some(m) = req.max_tokens {
        body["max_tokens"] = json!(m);
    }
    let mut rb = Request::post(&url);
    if let Some(k) = &req.api_key {
        if !k.is_empty() {
            rb = rb.header("Authorization", &format!("Bearer {k}"));
        }
    }
    let resp = rb
        .json(&body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| {
            format!(
                "{} request failed: Cannot connect to API: {e}",
                req.provider
            )
        })?;
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("{} returned a non-JSON response: {e}", req.provider))?;
    if let Some(err) = v.get("error") {
        return Err(format!("{} error: {err}", req.provider));
    }
    Ok(v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string())
}