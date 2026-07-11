use gloo_net::http::Request;
use serde_json::Value;
use tck_core::{CatalogModel, CommitRequest, CreateInstanceRequest, GitHubStatus, InstanceInfo, RepoEntry, RepoFile, RepoInfo, ScaffoldToRepoRequest};

pub async fn check_server_health() -> bool {
    let resp = Request::get("/api/health").send().await.ok();
    let Some(resp) = resp else { return false; };
    if !resp.ok() {
        return false;
    }
    let text = resp.text().await.ok();
    let Some(text) = text else { return false; };
    text.trim() == "ok"
}

pub async fn fetch_gh_status() -> Option<GitHubStatus> {
    let resp = Request::get("/api/github/status").send().await.ok()?;
    if !resp.ok() {
        return None;
    }
    resp.json::<GitHubStatus>().await.ok()
}

pub async fn fetch_model_catalog() -> Vec<CatalogModel> {
    let resp = Request::get("/api/ai/models").send().await.ok();
    let Some(resp) = resp else { return vec![]; };
    if !resp.ok() {
        return vec![];
    }
    resp.json::<Vec<CatalogModel>>().await.ok().unwrap_or_default()
}

pub async fn fetch_instances() -> Vec<InstanceInfo> {
    let resp = Request::get("/api/instances").send().await.ok();
    let Some(resp) = resp else { return vec![]; };
    if !resp.ok() {
        return vec![];
    }
    resp.json::<Vec<InstanceInfo>>().await.ok().unwrap_or_default()
}

pub async fn create_instance(req: &CreateInstanceRequest) -> Option<InstanceInfo> {
    let body = serde_json::to_string(req).ok()?;
    let req = Request::post("/api/instances")
        .header("Content-Type", "application/json")
        .body(body)
        .ok()?;
    req.send().await.ok()?.json::<InstanceInfo>().await.ok()
}

pub async fn write_stdin(id: &str, input: &str) -> bool {
    let body = serde_json::json!({"input": input}).to_string();
    let req = Request::post(&format!("/api/instances/{id}/stdin"))
        .header("Content-Type", "application/json")
        .body(body)
        .ok();
    let Some(req) = req else { return false; };
    req.send().await.map(|r| r.ok()).unwrap_or(false)
}

pub async fn delete_instance(id: &str) -> bool {
    Request::delete(&format!("/api/instances/{id}"))
        .send()
        .await
        .map(|r| r.ok())
        .unwrap_or(false)
}

pub fn instance_ws_url(id: &str) -> String {
    let loc = web_sys::window().unwrap().location();
    let proto = if loc.protocol().unwrap_or_default() == "https:" { "wss" } else { "ws" };
    let host = loc.host().unwrap_or_else(|_| "127.0.0.1:8080".into());
    format!("{proto}://{host}/ws/pty/{id}")
}

pub async fn fetch_repos() -> Result<Vec<RepoInfo>, String> {
    let resp = Request::get("/api/github/repos")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(api_err(resp).await);
    }
    resp.json::<Vec<RepoInfo>>()
        .await
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct ContentsResp {
    #[allow(dead_code)]
    pub kind: String,
    #[serde(default)]
    pub entries: Vec<RepoEntry>,
    #[serde(default)]
    pub file: Option<RepoFile>,
}

pub async fn fetch_contents(repo: &str, path: &str) -> Result<ContentsResp, String> {
    let url = format!(
        "/api/github/contents?repo={}&path={}",
        urlenc(repo),
        urlenc(path)
    );
    let resp = Request::get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(api_err(resp).await);
    }
    resp.json::<ContentsResp>().await.map_err(|e| e.to_string())
}

pub async fn commit_file(req: &CommitRequest) -> Result<String, String> {
    let resp = Request::put("/api/github/commit")
        .json(req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(api_err(resp).await);
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(v["sha"].as_str().unwrap_or_default().to_string())
}

pub async fn scaffold_repo(req: &ScaffoldToRepoRequest) -> Result<String, String> {
    let resp = Request::post("/api/scaffold/to-repo")
        .json(req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(api_err(resp).await);
    }
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(v["html_url"].as_str().unwrap_or_default().to_string())
}

async fn api_err(resp: gloo_net::http::Response) -> String {
    match resp.json::<Value>().await {
        Ok(v) => v
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("request failed")
            .to_string(),
        Err(_) => "request failed".to_string(),
    }
}

fn urlenc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}