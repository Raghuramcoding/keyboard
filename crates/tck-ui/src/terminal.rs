use dioxus::prelude::*;
use gloo_net::websocket::{futures::WebSocket, Message as WsMessage};
use futures_util::StreamExt;
use crate::state::AppState;
use crate::api::instance_ws_url;

pub fn sanitize_terminal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => match chars.next() {
                Some('[') => {
                    for n in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&n) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    while let Some(n) = chars.next() {
                        if n == '\u{07}' {
                            break;
                        }
                        if n == '\u{1b}' {
                            chars.next();
                            break;
                        }
                    }
                }
                Some('(') | Some(')') | Some('#') => {
                    chars.next();
                }
                _ => {}
            },
            '\n' | '\r' | '\t' => out.push(c),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {}
            c => out.push(c),
        }
    }
    out
}

pub fn ws_url() -> String {
    let loc = web_sys::window().unwrap().location();
    let proto = if loc.protocol().unwrap_or_default() == "https:" { "wss" } else { "ws" };
    let host = loc.host().unwrap_or_else(|_| "127.0.0.1:8080".into());
    format!("{proto}://{host}/ws/pty")
}

pub async fn connect_instance_terminal(state: AppState, id: String) {
    let ws_url = instance_ws_url(&id);
    let mut instances = state.instances;

    spawn(async move {
        match WebSocket::open(&ws_url) {
            Ok(ws) => {
                let (_write, mut read) = ws.split();
                let mut received = false;
                while let Some(item) = read.next().await {
                    let msg = match item {
                        Ok(m) => m,
                        Err(_) => break,
                    };
                    let text = match msg {
                        WsMessage::Text(t) => t,
                        WsMessage::Bytes(b) => String::from_utf8_lossy(&b).to_string(),
                    };
                    let sanitized = sanitize_terminal(&text);
                    let mut list = instances.peek().clone();
                    if let Some(inst) = list.iter_mut().find(|s| s.info.id == id) {
                        if !received {
                            received = true;
                            inst.term_connected = true;
                        }
                        inst.term_lines.push_str(&sanitized);
                        if inst.term_lines.len() > 120_000 {
                            inst.term_lines = inst.term_lines.split_off(inst.term_lines.len() - 100_000);
                        }
                    }
                    instances.set(list);
                }
                let mut list = instances.peek().clone();
                if let Some(inst) = list.iter_mut().find(|s| s.info.id == id) {
                    inst.term_connected = false;
                    if !received {
                        inst.term_lines.push_str("[T.C.K] Terminal offline — cannot reach server.\n");
                    }
                }
                instances.set(list);
            }
            Err(_) => {
                let mut list = instances.peek().clone();
                if let Some(inst) = list.iter_mut().find(|s| s.info.id == id) {
                    inst.term_connected = false;
                    inst.term_lines.push_str("[T.C.K] Failed to connect to terminal.\n");
                }
                instances.set(list);
            }
        }
    });
}