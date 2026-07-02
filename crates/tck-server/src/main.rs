//! T.C.K — TalentCloud Keyboard: native server.
//!
//! Serves the WebAssembly UI, proxies AI generation requests to providers
//! (Ollama / Claude / any OpenAI-compatible endpoint), and bridges a real
//! local shell to the browser over a WebSocket using a pseudo-terminal.

use axum::{
    body::Body,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{Read, Write};
use tck_core::{GenerateRequest, GenerateResponse};
use tower_http::services::{ServeDir, ServeFile};

mod github;

/// Base URL of the tck-ai service (Vercel AI SDK + models.dev).
/// Override with TCK_AI_BASE env var for different deployments.
fn tck_ai_base() -> String {
    std::env::var("TCK_AI_BASE").unwrap_or_else(|_| "http://127.0.0.1:3002".to_string())
}

#[tokio::main]
async fn main() {
    let dist = std::env::var("TCK_DIST").unwrap_or_else(|_| "crates/tck-ui/dist".to_string());
    let index = format!("{dist}/index.html");
    let static_service = ServeDir::new(&dist).fallback(ServeFile::new(index));

    let app = Router::new()
        // Legacy single-shot generate (kept for backward compat — proxies to tck-ai).
        .route("/api/generate", post(generate))
        // New Vercel AI SDK proxy to tck-ai.
        .route("/api/ai/generate", post(proxy_ai_generate))
        .route("/api/ai/models", get(proxy_ai_models))
        .route("/api/ai/providers", get(proxy_ai_providers))
        .route("/api/ai/health", get(proxy_ai_health))
        .route("/api/health", get(|| async { "ok" }))
        .route("/ws/pty", get(pty_ws))
        .merge(github::router())
        .fallback_service(static_service);

    let addr = std::env::var("TCK_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("T.C.K: cannot bind {addr}: {e}"));

    println!("T.C.K (TalentCloud Keyboard) server listening on http://{addr}");
    println!("Serving UI from: {dist}");
    axum::serve(listener, app).await.unwrap();
}

// ---------------- AI generation (proxied to tck-ai) ----------------

/// HTTP client shared across proxy calls.
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .build()
        .expect("failed to build HTTP client")
}

/// Legacy /api/generate — kept for backward compat, proxies to tck-ai.
async fn generate(Json(req): Json<GenerateRequest>) -> Json<GenerateResponse> {
    match proxy_to_tck_ai("/v1/generate", &req).await {
        Ok(resp) => Json(GenerateResponse {
            response: Some(resp.response),
            model: Some(resp.model),
            ..Default::default()
        }),
        Err(e) => Json(GenerateResponse {
            error: Some(e),
            ..Default::default()
        }),
    }
}

/// Proxy a GenerateRequest to tck-ai and return the parsed response.
async fn proxy_to_tck_ai(path: &str, req: &GenerateRequest) -> Result<AIGenerateResponse, String> {
    let base = tck_ai_base();
    let url = format!("{base}{path}");
    let body = serde_json::json!({
        "provider": req.provider,
        "model": req.model,
        "prompt": req.prompt,
        "system": req.system,
        "apiKey": req.api_key,
        "baseUrl": req.base_url,
        "temperature": req.temperature,
        "maxTokens": req.max_tokens,
    });

    let resp = client()
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Cannot reach tck-ai at {base}: {e}"))?;

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return Err(format!("tck-ai error ({path}): {err_body}"));
    }

    resp.json::<AIGenerateResponse>()
        .await
        .map_err(|e| format!("Bad response from tck-ai: {e}"))
}

/// Response shape from tck-ai's /v1/generate endpoint.
#[derive(serde::Deserialize)]
struct AIGenerateResponse {
    response: String,
    model: String,
}

/// POST /api/ai/generate — proxies to tck-ai /v1/generate.
async fn proxy_ai_generate(Json(req): Json<serde_json::Value>) -> Response {
    let base = tck_ai_base();
    let url = format!("{base}/v1/generate");
    let result = client()
        .post(&url)
        .json(&req)
        .send()
        .await;
    match result {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.bytes().await.unwrap_or_default();
            Response::builder()
                .status(status)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap_or_default()
        }
        Err(e) => Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"error":"Cannot reach tck-ai: {e}"}}"#
            )))
            .unwrap_or_default(),
    }
}

/// GET /api/ai/models — proxies to tck-ai /v1/models.
async fn proxy_ai_models() -> Response {
    proxy_get("/v1/models").await
}

/// GET /api/ai/providers — proxies to tck-ai /v1/providers.
async fn proxy_ai_providers() -> Response {
    proxy_get("/v1/providers").await
}

/// GET /api/ai/health — proxies to tck-ai /v1/health.
async fn proxy_ai_health() -> Response {
    proxy_get("/v1/health").await
}

/// Generic GET proxy helper.
async fn proxy_get(path: &str) -> Response {
    let base = tck_ai_base();
    let url = format!("{base}{path}");
    match client().get(&url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let body = resp.bytes().await.unwrap_or_default();
            Response::builder()
                .status(status)
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap_or_default()
        }
        Err(e) => Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"error":"Cannot reach tck-ai: {e}"}}"#
            )))
            .unwrap_or_default(),
    }
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

    // Launch the platform shell: PowerShell on Windows (fallback to cmd.exe);
    // on macOS/Linux honor the user's $SHELL (zsh by default on macOS), falling back to /bin/bash.
    let mut cmd = if cfg!(windows) {
        // Try pwsh (PowerShell 7+) first, then powershell.exe, then cmd.exe as fallback.
        if which::which("pwsh").is_ok() {
            CommandBuilder::new("pwsh")
        } else if which::which("powershell").is_ok() {
            CommandBuilder::new("powershell.exe")
        } else {
            CommandBuilder::new("cmd.exe")
        }
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
