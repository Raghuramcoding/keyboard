//! T.C.K — TalentCloud Keyboard: native server binary.
//!
//! Thin wrapper around the library. Calls `tck_server::serve_on_addr`
//! with the address from `TCK_ADDR` (default 127.0.0.1:3000) and the
//! WASM bundle path from `TCK_DIST` (default crates/tck-ui/dist).

#[tokio::main]
async fn main() {
    let dist = std::env::var("TCK_DIST").unwrap_or_else(|_| "crates/tck-ui/dist".to_string());
    let addr = std::env::var("TCK_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());

    let (_addr, _event_rx) = tck_server::serve_on_addr(&addr, &dist)
        .await
        .unwrap_or_else(|_| {
            eprintln!("T.C.K server failed to start");
            std::process::exit(1);
        });

    // Keep the process alive — the server runs on a spawned task.
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}
