//! T.C.K — TalentCloud Keyboard: native server.
//!
//! Serves the WebAssembly UI, proxies AI generation requests to providers
//! (Ollama / Claude / any OpenAI-compatible endpoint), and bridges a real
//! local shell to the browser over a WebSocket using a pseudo-terminal.

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde_json::{json, Value};
use std::io::{Read, Write};
use tck_core::{GenerateRequest, GenerateResponse};
use tower_http::services::{ServeDir, ServeFile};

#[tokio::main]
async fn main() {
    let dist = std::env::var("TCK_DIST").unwrap_or_else(|_| "crates/tck-ui/dist".to_string());
    let index = format!("{dist}/index.html");
    let static_service = ServeDir::new(&dist).fallback(ServeFile::new(index));

    let app = Router::new()
        .route("/api/generate", post(generate))
        .route("/api/health", get(|| async { "ok" }))
        .route("/ws/pty", get(pty_ws))
        .fallback_service(static_service);

    let addr = std::env::var("TCK_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("T.C.K: cannot bind {addr}: {e}"));

    println!("T.C.K (TalentCloud Keyboard) server listening on http://{addr}");
    println!("Serving UI from: {dist}");
    axum::serve(listener, app).await.unwrap();
}

// ---------------- AI generation ----------------

async fn generate(Json(req): Json<GenerateRequest>) -> Json<GenerateResponse> {
    let model = req.model.clone();
    let result = match req.provider.as_str() {
        "ollama" => gen_ollama(&req).await,
        "claude" => gen_claude(&req).await,
        _ => gen_openai_compatible(&req).await,
    };
    match result {
        Ok(text) => Json(GenerateResponse {
            response: Some(text),
            model: Some(model),
            ..Default::default()
        }),
        Err(e) => Json(GenerateResponse {
            error: Some(e),
            ..Default::default()
        }),
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .build()
        .expect("failed to build HTTP client")
}

async fn gen_ollama(req: &GenerateRequest) -> Result<String, String> {
    let host = req
        .base_url
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "http://localhost:11434".to_string());
    let url = format!("{}/api/generate", host.trim_end_matches('/'));
    let body = json!({
        "model": req.model,
        "prompt": req.prompt,
        "system": req.system.clone().unwrap_or_default(),
        "stream": false,
    });
    let resp = client()
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Cannot connect to Ollama ({host}). Is it running? {e}"))?;
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Bad Ollama response: {e}"))?;
    if let Some(err) = v.get("error") {
        return Err(format!("Ollama error: {}", err.as_str().unwrap_or("unknown")));
    }
    Ok(v["response"].as_str().unwrap_or_default().to_string())
}

async fn gen_claude(req: &GenerateRequest) -> Result<String, String> {
    let key = req.api_key.clone().unwrap_or_default();
    if key.is_empty() {
        return Err("No Claude API key configured (set it in Settings).".to_string());
    }
    let body = json!({
        "model": req.model,
        "max_tokens": req.max_tokens.unwrap_or(2048),
        "system": req.system.clone().unwrap_or_default(),
        "messages": [{ "role": "user", "content": req.prompt }],
    });
    let resp = client()
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Cannot connect to Claude API: {e}"))?;
    let v: Value = resp
        .json()
        .await
        .map_err(|e| format!("Bad Claude response: {e}"))?;
    if let Some(err) = v.get("error") {
        return Err(format!("Claude error: {err}"));
    }
    Ok(v["content"][0]["text"].as_str().unwrap_or_default().to_string())
}

async fn gen_openai_compatible(req: &GenerateRequest) -> Result<String, String> {
    let base = req.base_url.clone().unwrap_or_default();
    if base.is_empty() {
        return Err(format!("Provider \"{}\" has no base URL configured.", req.provider));
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
    let mut rb = client().post(&url);
    if let Some(k) = &req.api_key {
        if !k.is_empty() {
            rb = rb.bearer_auth(k);
        }
    }
    let resp = rb
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("{} request failed: Cannot connect to API: {e}", req.provider))?;
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

// ---------------- Terminal (PTY over WebSocket) ----------------

async fn pty_ws(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_pty)
}

async fn handle_pty(socket: WebSocket) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    let pty_system = native_pty_system();
    let pair = match pty_system.openpty(PtySize {
        rows: 30,
        cols: 110,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(p) => p,
        Err(e) => {
            let _ = ws_tx
                .send(Message::Text(format!("[T.C.K] failed to open PTY: {e}")))
                .await;
            return;
        }
    };

    // Launch the platform shell: PowerShell on Windows; on macOS/Linux honor the
    // user's $SHELL (zsh by default on macOS), falling back to /bin/bash.
    let mut cmd = if cfg!(windows) {
        CommandBuilder::new("powershell.exe")
    } else {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        CommandBuilder::new(shell)
    };
    if let Ok(cwd) = std::env::current_dir() {
        cmd.cwd(cwd);
    }
    let mut child = match pair.slave.spawn_command(cmd) {
        Ok(c) => c,
        Err(e) => {
            let _ = ws_tx
                .send(Message::Text(format!("[T.C.K] failed to spawn shell: {e}")))
                .await;
            return;
        }
    };

    // Forward PTY output -> browser. PTY reads are blocking, so do it on a
    // dedicated thread and shuttle bytes through a channel.
    let mut reader = pair.master.try_clone_reader().expect("clone pty reader");
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let send_task = tokio::spawn(async move {
        while let Some(chunk) = rx.recv().await {
            let text = String::from_utf8_lossy(&chunk).to_string();
            if ws_tx.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    });

    // Forward browser input -> PTY.
    let mut writer = pair.master.take_writer().expect("take pty writer");
    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Text(t) => {
                let _ = writer.write_all(t.as_bytes());
                let _ = writer.flush();
            }
            Message::Binary(b) => {
                let _ = writer.write_all(&b);
                let _ = writer.flush();
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    let _ = child.kill();
    send_task.abort();
}
