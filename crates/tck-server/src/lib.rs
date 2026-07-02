//! T.C.K — TalentCloud Keyboard: server library.
//!
//! Exposes `serve_on_addr` for embedding the server inside a desktop app.
//! The binary (`main.rs`) calls this function with env-var defaults.

mod github;
mod instances;

use axum::{
    body::Body,
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use instances::InstanceManager;
use std::sync::Arc;
use tck_core::{CreateInstanceRequest, GenerateRequest, GenerateResponse, InstanceInfo, UpdateInstanceRequest};
use tower_http::services::{ServeDir, ServeFile};

/// Base URL of the tck-ai service (Vercel AI SDK + models.dev).
/// Override with TCK_AI_BASE env var for different deployments.
fn tck_ai_base() -> String {
    std::env::var("TCK_AI_BASE").unwrap_or_else(|_| "http://127.0.0.1:3002".to_string())
}

/// Build and return the axum [`Router`] (with its [`InstanceManager`] inside).
pub fn build_router(dist_dir: &str) -> (Router, Arc<InstanceManager>) {
    let index = format!("{dist_dir}/index.html");
    let static_service = ServeDir::new(dist_dir).fallback(ServeFile::new(index));

    let im: Arc<InstanceManager> = Arc::new(InstanceManager::new());

    let app = Router::new()
        .route("/api/generate", post(generate))
        .route("/api/ai/generate", post(proxy_ai_generate))
        .route("/api/ai/models", get(proxy_ai_models))
        .route("/api/ai/providers", get(proxy_ai_providers))
        .route("/api/ai/health", get(proxy_ai_health))
        .route("/api/health", get(|| async { "ok" }))
        .route("/ws/pty", get(pty_ws))
        .route("/api/instances", get(list_instances).post(create_instance))
        .route("/api/instances/{id}", get(get_instance).patch(update_instance).delete(delete_instance))
        .route("/api/instances/{id}/stdin", post(instance_stdin))
        .route("/ws/pty/{id}", get(pty_ws_multi))
        .with_state(im.clone())
        .merge(github::router())
        .fallback_service(static_service);

    (app, im)
}

/// Start the T.C.K server on `addr`, serving static files from `dist_dir`.
///
/// This spawns the server loop on the current tokio runtime and returns the
/// bound [`std::net::SocketAddr`] immediately (useful when `addr` includes
/// port 0 for OS-assignment). The server runs until the tokio runtime shuts
/// down.
pub async fn serve_on_addr(addr: &str, dist_dir: &str) -> Result<std::net::SocketAddr, ()> {
    let (app, _im) = build_router(dist_dir);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| eprintln!("T.C.K: cannot bind {addr}: {e}"))?;

    let local_addr = listener.local_addr()
        .map_err(|e| eprintln!("T.C.K: failed to get bound address: {e}"))?;

    println!("T.C.K (TalentCloud Keyboard) server listening on http://{local_addr}");
    println!("Serving UI from: {dist_dir}");

    // Spawn the server loop in background so we can return immediately.
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("T.C.K: server error: {e}");
        }
    });

    Ok(local_addr)
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

// ---------------- Multi-instance dashboard API ----------------

async fn list_instances(State(im): State<Arc<InstanceManager>>) -> Json<Vec<InstanceInfo>> {
    Json(im.list())
}

async fn create_instance(
    State(im): State<Arc<InstanceManager>>,
    Json(req): Json<CreateInstanceRequest>,
) -> Response {
    match im.create(&req) {
        Ok(info) => (StatusCode::CREATED, Json(info)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e}))).into_response(),
    }
}

async fn get_instance(
    State(im): State<Arc<InstanceManager>>,
    Path(id): Path<String>,
) -> Response {
    match im.get(&id) {
        Some(info) => Json(info).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not found"}))).into_response(),
    }
}

async fn update_instance(
    State(im): State<Arc<InstanceManager>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateInstanceRequest>,
) -> Response {
    if let Some(label) = &req.label {
        if let Err(e) = im.rename(&id, label) {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e}))).into_response();
        }
    }
    match im.get(&id) {
        Some(info) => Json(info).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not found"}))).into_response(),
    }
}

async fn delete_instance(
    State(im): State<Arc<InstanceManager>>,
    Path(id): Path<String>,
) -> Response {
    match im.kill(&id) {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e}))).into_response(),
    }
}

async fn instance_stdin(
    State(im): State<Arc<InstanceManager>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let input = body.get("input").and_then(|v| v.as_str()).unwrap_or("");
    match im.write_stdin(&id, input) {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e}))).into_response(),
    }
}

/// WebSocket handler for a single multi-instance PTY.
async fn pty_ws_multi(
    State(im): State<Arc<InstanceManager>>,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_multi_pty(socket, im, id))
}

async fn handle_multi_pty(socket: WebSocket, im: Arc<InstanceManager>, id: String) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Attach to the running instance's output stream.
    let (replay, mut rx, writer) = match im.attach(&id) {
        Ok(v) => v,
        Err(e) => {
            let _ = ws_tx
                .send(Message::Text(format!("[T.C.K] attach failed: {e}")))
                .await;
            return;
        }
    };

    // Replay buffered output.
    if !replay.is_empty() {
        let text = String::from_utf8_lossy(&replay).to_string();
        if ws_tx.send(Message::Text(text)).await.is_err() {
            return;
        }
    }

    // Forward live output → WS.
    let send_task = tokio::spawn(async move {
        while let Ok(chunk) = rx.recv().await {
            let text = String::from_utf8_lossy(&chunk).to_string();
            if ws_tx.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    });

    // Forward WS input → PTY stdin.
    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Text(t) => {
                let mut w = writer.lock().unwrap();
                let _ = w.write_all(t.as_bytes());
                let _ = w.flush();
            }
            Message::Binary(b) => {
                let mut w = writer.lock().unwrap();
                let _ = w.write_all(&b);
                let _ = w.flush();
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    send_task.abort();
}

// ---------------- Terminal (PTY over WebSocket) ----------------

async fn pty_ws(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_pty)
}

async fn handle_pty(socket: WebSocket) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    let pty_system = portable_pty::native_pty_system();
    let pair = match pty_system.openpty(portable_pty::PtySize {
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

    let mut cmd = if cfg!(windows) {
        if which::which("pwsh").is_ok() {
            portable_pty::CommandBuilder::new("pwsh")
        } else if which::which("powershell").is_ok() {
            portable_pty::CommandBuilder::new("powershell.exe")
        } else {
            portable_pty::CommandBuilder::new("cmd.exe")
        }
    } else {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        portable_pty::CommandBuilder::new(shell)
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
