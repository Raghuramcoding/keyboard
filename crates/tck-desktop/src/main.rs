//! T.C.K — TalentCloud Keyboard: desktop app.
//!
//! Embeds the axum server and WASM bundle into a native desktop window
//! (WebView2 on Windows, WebKit on macOS, WebKitGTK on Linux).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rust_embed::RustEmbed;
use std::fs;
use std::io::Write;
use std::sync::mpsc;
use std::thread;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use url::Url;
use wry::WebViewBuilder;

/// Embedded WASM static assets (built by `trunk build` from `tck-ui`).
#[derive(RustEmbed)]
#[folder = "../tck-ui/dist"]
struct Assets;

fn extract_assets() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("failed to create temp directory");
    for path in Assets::iter() {
        let disk_path = dir.path().join(path.as_ref());
        if let Some(parent) = disk_path.parent() {
            fs::create_dir_all(parent).expect("failed to create temp subdirectory");
        }
        let data = Assets::get(&path).expect("embedded asset not found").data;
        let mut f = fs::File::create(&disk_path).expect("failed to create asset file");
        f.write_all(&data).expect("failed to write asset data");
    }
    dir
}

fn main() {
    // 1. Extract embedded WASM assets to a temp directory.
    let _tmp = extract_assets();
    let dist_dir = _tmp.path().to_str().unwrap().to_string();

    // 2. Start the tokio runtime + server on a background thread.
    //    Bind to port 0 so the OS assigns a free port.
    let (addr_tx, addr_rx) = mpsc::channel();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        rt.block_on(async {
            match tck_server::serve_on_addr("127.0.0.1:0", &dist_dir).await {
                Ok(addr) => {
                    let _ = addr_tx.send(addr);
                }
                Err(_) => {
                    eprintln!("T.C.K server failed to start");
                    std::process::exit(1);
                }
            }
            // Keep the runtime alive so the server keeps running.
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
    });

    // 3. Wait for the server to be ready, then open the webview.
    let server_addr = addr_rx.recv().expect("failed to get server address");
    let url = format!("http://{server_addr}");

    let event_loop = EventLoopBuilder::new().build();
    let window = WindowBuilder::new()
        .with_title("T.C.K — TalentCloud Keyboard")
        .with_inner_size(tao::dpi::LogicalSize::new(1200, 800))
        .build(&event_loop)
        .expect("failed to create window");

    let _webview = WebViewBuilder::new(&window)
        .with_url(&url)
        .expect("invalid URL")
        .build()
        .expect("failed to create webview");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let tao::event::Event::WindowEvent { event, .. } = event {
            if matches!(event, tao::event::WindowEvent::CloseRequested) {
                *control_flow = ControlFlow::Exit;
            }
        }
    });
}
