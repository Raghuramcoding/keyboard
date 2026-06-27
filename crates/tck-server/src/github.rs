//! GitHub integration for T.C.K: OAuth (an OAuth App) + a thin proxy over the
//! GitHub REST API + scaffolding into new repos.
//!
//! The OAuth `client_secret` and the user's access token live ONLY here, in the
//! Rust server. The browser holds just an httpOnly session cookie; it never sees
//! the token. Configure via env:
//!   TCK_GITHUB_CLIENT_ID, TCK_GITHUB_CLIENT_SECRET
//!   TCK_GITHUB_CALLBACK   (optional; otherwise derived from the request Host)

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post, put},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use rand::Rng;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tck_core::{
    scaffold_templates, CommitRequest, GitHubStatus, RepoEntry, RepoFile, RepoInfo,
    ScaffoldToRepoRequest,
};

#[derive(Clone, Default)]
pub struct GhState {
    inner: Arc<Mutex<GhInner>>,
}

#[derive(Default)]
struct GhInner {
    /// session id -> (token, login)
    sessions: HashMap<String, (String, String)>,
    /// outstanding OAuth `state` nonces (CSRF protection)
    states: HashMap<String, ()>,
}

pub fn router() -> Router {
    Router::new()
        .route("/auth/github/login", get(login))
        .route("/auth/github/callback", get(callback))
        .route("/auth/github/logout", post(logout))
        .route("/api/github/status", get(status))
        .route("/api/github/repos", get(repos))
        .route("/api/github/contents", get(contents))
        .route("/api/github/commit", put(commit))
        .route("/api/github/create-repo", post(create_repo))
        .route("/api/scaffold/to-repo", post(scaffold_to_repo))
        .with_state(GhState::default())
}

// ---------- config ----------

fn client_id() -> Option<String> {
    std::env::var("TCK_GITHUB_CLIENT_ID").ok().filter(|s| !s.is_empty())
}
fn client_secret() -> Option<String> {
    std::env::var("TCK_GITHUB_CLIENT_SECRET").ok().filter(|s| !s.is_empty())
}
fn configured() -> bool {
    client_id().is_some() && client_secret().is_some()
}

fn callback_url(headers: &HeaderMap) -> String {
    if let Ok(cb) = std::env::var("TCK_GITHUB_CALLBACK") {
        if !cb.is_empty() {
            return cb;
        }
    }
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("127.0.0.1:3000");
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("http");
    format!("{scheme}://{host}/auth/github/callback")
}

fn random_token() -> String {
    let mut b = [0u8; 24];
    rand::thread_rng().fill(&mut b[..]);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Percent-encode for a query string (RFC 3986 unreserved set passes through).
fn pct(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

// ---------- OAuth ----------

async fn login(State(st): State<GhState>, headers: HeaderMap) -> Response {
    let Some(cid) = client_id() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "GitHub OAuth is not configured on this server.",
        )
            .into_response();
    };
    if client_secret().is_none() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "GitHub OAuth is not configured on this server.",
        )
            .into_response();
    }
    let state = random_token();
    st.inner.lock().unwrap().states.insert(state.clone(), ());
    let redirect_uri = callback_url(&headers);
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope=repo&state={}",
        pct(&cid),
        pct(&redirect_uri),
        pct(&state),
    );
    Redirect::to(&url).into_response()
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn callback(
    State(st): State<GhState>,
    jar: CookieJar,
    headers: HeaderMap,
    Query(q): Query<CallbackQuery>,
) -> Response {
    if let Some(e) = q.error {
        return (StatusCode::BAD_REQUEST, format!("GitHub OAuth error: {e}")).into_response();
    }
    let (Some(code), Some(state)) = (q.code, q.state) else {
        return (StatusCode::BAD_REQUEST, "Missing code/state").into_response();
    };
    if st.inner.lock().unwrap().states.remove(&state).is_none() {
        return (StatusCode::BAD_REQUEST, "Invalid OAuth state").into_response();
    }
    let (cid, secret) = (client_id().unwrap(), client_secret().unwrap());
    let redirect_uri = callback_url(&headers);
    let token = match exchange_code(&cid, &secret, &code, &redirect_uri).await {
        Ok(t) => t,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("Token exchange failed: {e}")).into_response(),
    };
    let login = gh_login(&token).await.unwrap_or_else(|_| "unknown".to_string());
    let sid = random_token();
    st.inner.lock().unwrap().sessions.insert(sid.clone(), (token, login));
    let cookie = Cookie::build(("tck_sess", sid))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();
    (jar.add(cookie), Redirect::to("/")).into_response()
}

async fn logout(State(st): State<GhState>, jar: CookieJar) -> Response {
    if let Some(c) = jar.get("tck_sess") {
        st.inner.lock().unwrap().sessions.remove(c.value());
    }
    let removal = Cookie::build(("tck_sess", "")).path("/").build();
    (jar.remove(removal), Json(json!({ "ok": true }))).into_response()
}

async fn exchange_code(cid: &str, secret: &str, code: &str, redirect_uri: &str) -> Result<String, String> {
    let body = json!({
        "client_id": cid, "client_secret": secret,
        "code": code, "redirect_uri": redirect_uri,
    });
    let resp = reqwest::Client::new()
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    if let Some(err) = v.get("error_description").and_then(|e| e.as_str()) {
        return Err(err.to_string());
    }
    v["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "no access_token in response".to_string())
}

// ---------- session helpers ----------

fn session(st: &GhState, jar: &CookieJar) -> Option<(String, String)> {
    let sid = jar.get("tck_sess")?.value().to_string();
    st.inner.lock().unwrap().sessions.get(&sid).cloned()
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "Not connected to GitHub. Click \"Connect GitHub\" first." })),
    )
        .into_response()
}

// ---------- GitHub REST helpers ----------

async fn gh_request(method: reqwest::Method, token: &str, url: &str, body: Option<Value>) -> Result<Value, String> {
    let mut rb = reqwest::Client::new()
        .request(method, url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "T.C.K")
        .header("Accept", "application/vnd.github+json");
    if let Some(b) = body {
        rb = rb.json(&b);
    }
    let resp = rb.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let v: Value = resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        let msg = v.get("message").and_then(|m| m.as_str()).unwrap_or("GitHub API error");
        return Err(format!("GitHub API {}: {msg}", status.as_u16()));
    }
    Ok(v)
}

async fn gh_login(token: &str) -> Result<String, String> {
    let v = gh_request(reqwest::Method::GET, token, "https://api.github.com/user", None).await?;
    Ok(v["login"].as_str().unwrap_or("unknown").to_string())
}

// ---------- API handlers ----------

async fn status(State(st): State<GhState>, jar: CookieJar) -> Json<GitHubStatus> {
    let sess = session(&st, &jar);
    Json(GitHubStatus {
        configured: configured(),
        connected: sess.is_some(),
        login: sess.map(|(_, l)| l),
    })
}

async fn repos(State(st): State<GhState>, jar: CookieJar) -> Response {
    let Some((token, _)) = session(&st, &jar) else {
        return unauthorized();
    };
    match gh_request(
        reqwest::Method::GET,
        &token,
        "https://api.github.com/user/repos?per_page=100&sort=updated",
        None,
    )
    .await
    {
        Ok(v) => {
            let list: Vec<RepoInfo> = v
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|r| RepoInfo {
                            full_name: r["full_name"].as_str().unwrap_or_default().to_string(),
                            default_branch: r["default_branch"].as_str().unwrap_or("main").to_string(),
                            private: r["private"].as_bool().unwrap_or(false),
                            description: r["description"].as_str().map(|s| s.to_string()),
                        })
                        .collect()
                })
                .unwrap_or_default();
            Json(list).into_response()
        }
        Err(e) => (StatusCode::BAD_GATEWAY, Json(json!({ "error": e }))).into_response(),
    }
}

#[derive(Deserialize)]
struct ContentsQuery {
    repo: String,
    #[serde(default)]
    path: String,
}

async fn contents(State(st): State<GhState>, jar: CookieJar, Query(q): Query<ContentsQuery>) -> Response {
    let Some((token, _)) = session(&st, &jar) else {
        return unauthorized();
    };
    let url = format!(
        "https://api.github.com/repos/{}/contents/{}",
        q.repo,
        q.path.trim_start_matches('/')
    );
    match gh_request(reqwest::Method::GET, &token, &url, None).await {
        Ok(v) => {
            if let Some(arr) = v.as_array() {
                let entries: Vec<RepoEntry> = arr
                    .iter()
                    .map(|e| RepoEntry {
                        name: e["name"].as_str().unwrap_or_default().to_string(),
                        path: e["path"].as_str().unwrap_or_default().to_string(),
                        kind: e["type"].as_str().unwrap_or("file").to_string(),
                        sha: e["sha"].as_str().unwrap_or_default().to_string(),
                    })
                    .collect();
                Json(json!({ "kind": "dir", "entries": entries })).into_response()
            } else {
                let raw = v["content"].as_str().unwrap_or_default();
                let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
                let content = B64
                    .decode(cleaned.as_bytes())
                    .ok()
                    .and_then(|b| String::from_utf8(b).ok())
                    .unwrap_or_else(|| "[binary file — not shown]".to_string());
                let file = RepoFile {
                    repo: q.repo.clone(),
                    path: q.path.clone(),
                    content,
                    sha: v["sha"].as_str().unwrap_or_default().to_string(),
                };
                Json(json!({ "kind": "file", "file": file })).into_response()
            }
        }
        Err(e) => (StatusCode::BAD_GATEWAY, Json(json!({ "error": e }))).into_response(),
    }
}

async fn commit(State(st): State<GhState>, jar: CookieJar, Json(req): Json<CommitRequest>) -> Response {
    let Some((token, _)) = session(&st, &jar) else {
        return unauthorized();
    };
    let url = format!(
        "https://api.github.com/repos/{}/contents/{}",
        req.repo,
        req.path.trim_start_matches('/')
    );
    let mut body = json!({
        "message": if req.message.is_empty() { "Update via T.C.K".to_string() } else { req.message.clone() },
        "content": B64.encode(req.content.as_bytes()),
    });
    if let Some(sha) = req.sha.filter(|s| !s.is_empty()) {
        body["sha"] = json!(sha);
    }
    match gh_request(reqwest::Method::PUT, &token, &url, Some(body)).await {
        Ok(v) => Json(json!({
            "ok": true,
            "sha": v["content"]["sha"],
            "html_url": v["content"]["html_url"],
        }))
        .into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, Json(json!({ "error": e }))).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateRepoReq {
    name: String,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    description: Option<String>,
}

async fn create_repo(State(st): State<GhState>, jar: CookieJar, Json(req): Json<CreateRepoReq>) -> Response {
    let Some((token, _)) = session(&st, &jar) else {
        return unauthorized();
    };
    match create_repo_inner(&token, &req.name, req.private, req.description.as_deref()).await {
        Ok(repo) => Json(repo).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, Json(json!({ "error": e }))).into_response(),
    }
}

async fn create_repo_inner(
    token: &str,
    name: &str,
    private: bool,
    description: Option<&str>,
) -> Result<Value, String> {
    let body = json!({
        "name": name, "private": private, "auto_init": false,
        "description": description.unwrap_or("Created with T.C.K"),
    });
    let v = gh_request(reqwest::Method::POST, token, "https://api.github.com/user/repos", Some(body)).await?;
    Ok(json!({
        "full_name": v["full_name"],
        "html_url": v["html_url"],
        "default_branch": v["default_branch"],
    }))
}

async fn scaffold_to_repo(
    State(st): State<GhState>,
    jar: CookieJar,
    Json(req): Json<ScaffoldToRepoRequest>,
) -> Response {
    let Some((token, _)) = session(&st, &jar) else {
        return unauthorized();
    };
    let Some(tpl) = scaffold_templates().into_iter().find(|t| t.id == req.template) else {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "Unknown template" }))).into_response();
    };
    // 1) create the repo
    let repo = match create_repo_inner(&token, &req.repo_name, req.private, Some(&tpl.description)).await {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_GATEWAY, Json(json!({ "error": e }))).into_response(),
    };
    let full_name = repo["full_name"].as_str().unwrap_or_default().to_string();
    // 2) commit each file
    for file in &tpl.files {
        let url = format!("https://api.github.com/repos/{full_name}/contents/{}", file.path);
        let body = json!({
            "message": format!("Scaffold {} via T.C.K", file.path),
            "content": B64.encode(file.content.as_bytes()),
        });
        if let Err(e) = gh_request(reqwest::Method::PUT, &token, &url, Some(body)).await {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": format!("created repo but failed writing {}: {e}", file.path), "html_url": repo["html_url"] })),
            )
                .into_response();
        }
    }
    Json(json!({
        "ok": true,
        "full_name": full_name,
        "html_url": repo["html_url"],
        "files": tpl.files.len(),
    }))
    .into_response()
}
