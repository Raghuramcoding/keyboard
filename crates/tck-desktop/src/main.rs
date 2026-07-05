//! T.C.K — TalentCloud Keyboard: desktop app.
//!
//! Embeds the axum server and WASM bundle into a native desktop window
//! (WebView2 on Windows, WebKit on macOS, WebKitGTK on Linux).
//!
//! Features:
//! - Single-instance lock via OS temp directory PID file
//! - Registers `tck://` custom protocol so the browser can launch this app
//! - System tray icon (blue circle) with Show/Hide and Quit
//! - Session persistence to `$CONFIG_DIR/tck/sessions/` on exit
//! - Old session clean-up (files older than 7 days)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rust_embed::RustEmbed;
use std::fs;
use std::io::Write;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use tray_icon::TrayIconBuilder;
use wry::WebViewBuilder;

// ---------------------------------------------------------------------------
// Embedded WASM assets (built by `trunk build` from `tck-ui`)
// ---------------------------------------------------------------------------

#[derive(RustEmbed)]
#[folder = "../tck-ui/dist"]
struct Assets;

// ---------------------------------------------------------------------------
// Custom event type for the tao event loop (cross-thread signaling)
// ---------------------------------------------------------------------------

/// Events sent to the event loop from background threads.
enum DesktopEvent {
    /// Bring the main window to the front.  Sent by the protocol-handler
    /// signaling thread when a second instance was launched via `tck://`.
    FocusWindow,
}

// ---------------------------------------------------------------------------
// Custom protocol handler registration (tck://)
// ---------------------------------------------------------------------------

/// Register `tck://` as a custom protocol so clicking links like
/// `tck://launch` in the browser opens this desktop app.
///
/// On **Windows** this writes to the current-user registry hive under
/// `HKCU\Software\Classes\tck`.  On **macOS** the app bundle's
/// `Info.plist` would need `CFBundleURLSchemes`; on **Linux** a
/// `.desktop` file with `MimeType=x-scheme-handler/tck`.
///
/// This is called once at startup and writes only if the registry key
/// does not already point to this executable.
fn register_protocol_handler() {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return,
    };
    let exe_path = exe.to_string_lossy().to_string();
    // Forward-slash variant for migration: earlier releases wrote forward
    // slashes into the registry, so we check both to avoid re-registration.
    #[cfg(target_os = "windows")]
    let exe_path_fwd = exe_path.replace('\\', "/");

    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use winreg::enums::*;
        use winreg::RegKey;

        let _widen = |s: &str| -> Vec<u16> {
            OsStr::new(s)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect()
        };

        // Try to open the protocol key — if it exists and points to us, skip.
        let classes = match RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags("Software\\Classes\\tck", KEY_READ)
        {
            Ok(key) => {
                if let Ok(val) = key.get_value::<String, _>("") {
                    if val == "URL:T.C.K Protocol" {
                        if let Ok(cmd_key) = key.open_subkey("shell\\open\\command") {
                            if let Ok(cmd) = cmd_key.get_value::<String, _>("") {
                                if cmd.contains(&exe_path) || cmd.contains(&exe_path_fwd) {
                                    return; // Already registered to this exe.
                                }
                            }
                        }
                    }
                }
                key
            }
            Err(_) => {
                // Key doesn't exist — create it.
                RegKey::predef(HKEY_CURRENT_USER)
                    .create_subkey("Software\\Classes\\tck")
                    .expect("failed to create tck protocol key")
                    .0
            }
        };

        // Set default value.
        classes
            .set_value("", &"URL:T.C.K Protocol")
            .expect("failed to set protocol name");

        // Mark as URL protocol.
        classes
            .set_value("URL Protocol", &"")
            .expect("failed to set URL Protocol");

        // Create shell\open\command subkey.
        let (cmd_key, _) = classes
            .create_subkey("shell\\open\\command")
            .expect("failed to create command subkey");

        // Command: "C:\path\to\tck-desktop.exe" "%1"
        let cmd_line = format!("\"{}\" \"%1\"", exe_path);
        cmd_key
            .set_value("", &cmd_line)
            .expect("failed to set command");

        // Notify the shell to refresh its icon cache for this protocol.
        // SAFETY: SHChangeNotify is safe to call with these null-ish arguments.
        unsafe {
            extern "system" {
                fn SHChangeNotify(
                    wEventId: u32,
                    uFlags: u32,
                    dwItem1: *const std::ffi::c_void,
                    dwItem2: *const std::ffi::c_void,
                );
            }
            // SHCNE_ASSOCCHANGED = 0x08000000, SHCNF_IDLIST = 0x0000
            SHChangeNotify(0x08000000, 0x0000, std::ptr::null(), std::ptr::null());
        }

        println!("T.C.K: registered tck:// protocol handler");
    }

    #[cfg(target_os = "macos")]
    {
        // macOS registration happens via CFBundleURLSchemes in Info.plist,
        // not at runtime.  Log a reminder for distribution packaging.
        #[cfg(debug_assertions)]
        eprintln!(
            "T.C.K: macOS — embed CFBundleURLSchemes in Info.plist for tck:// support."
        );
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, register via xdg-mime or a local .desktop file.
        let desktop_entry = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=T.C.K\n\
             Exec={} %u\n\
             MimeType=x-scheme-handler/tck;\n\
             NoDisplay=true\n",
            exe_path
        );
        let data_home = std::env::var("XDG_DATA_HOME")
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
                format!("{home}/.local/share")
            });
        let apps_dir = std::path::Path::new(&data_home).join("applications");
        let _ = fs::create_dir_all(&apps_dir);
        let desktop_path = apps_dir.join("tck.desktop");
        if fs::write(&desktop_path, &desktop_entry).is_ok() {
            // Register with xdg-mime (best-effort).
            let _ = std::process::Command::new("xdg-mime")
                .args(["default", "tck.desktop", "x-scheme-handler/tck"])
                .output();
            println!("T.C.K: registered tck:// protocol handler (Linux)");
        }
    }
}

// ---------------------------------------------------------------------------
// Single-instance lock
// ---------------------------------------------------------------------------

/// Try to acquire a single-instance lock by writing our PID to
/// `%TEMP%/tck-desktop.lock`.  If the lock exists and the owning process is
/// still alive, returns `Err`.
fn acquire_single_instance_lock() -> Result<(), String> {
    let lock_path = std::env::temp_dir().join("tck-desktop.lock");

    // Check existing lock.
    if let Ok(content) = fs::read_to_string(&lock_path) {
        if let Ok(pid) = content.trim().parse::<u32>() {
            if process_is_alive(pid) {
                return Err(format!(
                    "Another instance is already running (PID {pid})."
                ));
            }
        }
        // Stale lock — remove it.
        let _ = fs::remove_file(&lock_path);
    }

    // Write our PID.
    fs::write(&lock_path, std::process::id().to_string())
        .map_err(|e| format!("Cannot write lock file: {e}"))
}

/// Platform-specific check: is `pid` a running process?
///
/// Uses direct Win32 FFI (kernel32) to avoid a dependency on the `windows` crate.
#[cfg(target_os = "windows")]
fn process_is_alive(pid: u32) -> bool {
    // Type alias for raw handle pointer.
    // This matches the Win32 HANDLE typedef and is used in the FFI declarations below.
    #[allow(clippy::upper_case_acronyms)]
    type HANDLE = *mut std::ffi::c_void;

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000u32;
    const STILL_ACTIVE: u32 = 259u32;
    const ERROR_ACCESS_DENIED: u32 = 5u32;

    extern "system" {
        fn OpenProcess(
            dwDesiredAccess: u32,
            bInheritHandle: i32,
            dwProcessId: u32,
        ) -> HANDLE;
        fn GetExitCodeProcess(hProcess: HANDLE, lpExitCode: *mut u32) -> i32;
        fn CloseHandle(hObject: HANDLE) -> i32;
        fn GetLastError() -> u32;
    }

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            // ERROR_ACCESS_DENIED means the process exists but we cannot
            // open it — still counts as alive for our single-instance check.
            return GetLastError() == ERROR_ACCESS_DENIED;
        }
        let mut exit_code: u32 = 0;
        let ok = GetExitCodeProcess(handle, &mut exit_code as *mut u32);
        CloseHandle(handle);
        ok != 0 && exit_code == STILL_ACTIVE
    }
}

#[cfg(not(target_os = "windows"))]
fn process_is_alive(pid: u32) -> bool {
    // SAFETY: kill with signal 0 is a standard existence probe on POSIX.
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// Platform-specific warning dialog for duplicate instance.
#[cfg(target_os = "windows")]
fn show_duplicate_instance_dialog() {
    extern "system" {
        fn MessageBoxA(
            hWnd: *mut std::ffi::c_void,
            lpText: *const std::os::raw::c_char,
            lpCaption: *const std::os::raw::c_char,
            uType: u32,
        ) -> i32;
    }

    // SAFETY: string literals are null-terminated in Rust and statically
    // allocated, so passing them to MessageBoxA is safe.
    unsafe {
        MessageBoxA(
            std::ptr::null_mut(),
            c"T.C.K is already running.\n".as_ptr(),
            c"T.C.K".as_ptr(),
            0x00000040u32, // MB_OK | MB_ICONINFORMATION
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn show_duplicate_instance_dialog() {
    // On non-Windows platforms we rely on the stderr message shown in main().
}

/// Try to bring an existing T.C.K window to the foreground.
///
/// Called when a second instance is launched via `tck://` protocol.
/// The first instance wrote its PID to the lock file — we can use that
/// to find its window and focus it.
///
/// On Windows this attempts `SetForegroundWindow` (which may be denied by
/// the OS) AND writes a trigger file that the first instance polls every
/// 2 s, so the first instance can bring itself to front without permission
/// issues.
fn focus_existing_window() {
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn EnumWindows(
                lpEnumFunc: Option<
                    unsafe extern "system" fn(
                        *mut std::ffi::c_void,
                        *mut std::ffi::c_void,
                    ) -> i32,
                >,
                lParam: *mut std::ffi::c_void,
            ) -> i32;
            fn GetWindowThreadProcessId(
                hWnd: *mut std::ffi::c_void,
                lpdwProcessId: *mut u32,
            ) -> u32;
            fn SetForegroundWindow(hWnd: *mut std::ffi::c_void) -> i32;
            fn ShowWindow(hWnd: *mut std::ffi::c_void, nCmdShow: i32) -> i32;
        }
        const SW_RESTORE: i32 = 9;

        // Read the lock file to get the PID of the running instance.
        let lock_path = std::env::temp_dir().join("tck-desktop.lock");
        let pid = fs::read_to_string(&lock_path)
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        if pid == 0 {
            return;
        }

        static mut FOUND: *mut std::ffi::c_void = std::ptr::null_mut();
        static mut TARGET_PID: u32 = 0;

        // SAFETY: we hold a single-threaded lock (the second instance exits
        // immediately after calling focus_existing_window), so these statics
        // are accessed from at most one thread.
        unsafe {
            TARGET_PID = pid;

            extern "system" fn enum_proc(
                hwnd: *mut std::ffi::c_void,
                _lparam: *mut std::ffi::c_void,
            ) -> i32 {
                let mut found_pid: u32 = 0;
                // SAFETY: EnumWindows is synchronous, so this callback runs
                // in the same thread as the calling code.
                unsafe {
                    GetWindowThreadProcessId(hwnd, &mut found_pid as *mut u32);
                    if found_pid == TARGET_PID {
                        FOUND = hwnd;
                        return 0; // Stop enumeration.
                    }
                }
                1 // Continue.
            }

            let _ = EnumWindows(Some(enum_proc), std::ptr::null_mut());
            if !FOUND.is_null() {
                // Restore the window first (un-minimize + make visible),
                // then try to set it as the foreground window.
                ShowWindow(FOUND, SW_RESTORE);
                SetForegroundWindow(FOUND);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On non-Windows platforms we simply exit — the user can alt-tab.
    }

    // --- Fallback: signal the first instance via a trigger file ---
    //
    // Windows often denies SetForegroundWindow to a freshly-launched
    // process.  The trigger file is polled by a background thread in the
    // first instance, which can reliably set its own window as foreground.
    let trigger = std::env::temp_dir().join("tck-desktop.focus");
    let _ = fs::write(&trigger, "1");
}

// ---------------------------------------------------------------------------
// System tray
// ---------------------------------------------------------------------------

/// Build a 32x32 RGBA blue circle fallback tray icon.  Returns the RGBA
/// pixel buffer and the dimensions.
fn blue_circle_rgba() -> Vec<u8> {
    let w = 32u32;
    let h = 32u32;
    let cx = 15.5f64;
    let cy = 15.5f64;
    let r = 15.0f64;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);

    for y in 0..h {
        for x in 0..w {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            if dx * dx + dy * dy <= r * r {
                rgba.extend_from_slice(&[52, 120, 246, 255]); // blue
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]); // transparent
            }
        }
    }
    rgba
}

// ---------------------------------------------------------------------------
// Asset extraction
// ---------------------------------------------------------------------------

/// Extract all embedded WASM assets to a temporary directory and return the
/// directory handle (which keeps the files alive).
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

// ---------------------------------------------------------------------------
// Session persistence
// ---------------------------------------------------------------------------

/// Save a session record to `$CONFIG_DIR/tck/sessions/`.
fn save_session(addr: &str) {
    let config_dir = match dirs::config_dir() {
        Some(d) => d,
        None => return,
    };

    let sessions_dir = config_dir.join("tck").join("sessions");
    if fs::create_dir_all(&sessions_dir).is_err() {
        return;
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let session_file = sessions_dir.join(format!("{timestamp}.session"));
    let _ = fs::write(
        &session_file,
        format!("version=1\naddr={addr}\ntimestamp={timestamp}\n"),
    );
}

/// Remove session files older than 7 days from `$CONFIG_DIR/tck/sessions/`.
fn clean_old_sessions() {
    let config_dir = match dirs::config_dir() {
        Some(d) => d,
        None => return,
    };

    let sessions_dir = config_dir.join("tck").join("sessions");
    if !sessions_dir.is_dir() {
        return;
    }

    let now = std::time::SystemTime::now();
    let week_secs = 7u64 * 24 * 60 * 60;

    if let Ok(dir) = fs::read_dir(&sessions_dir) {
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("session") {
                continue;
            }
            if let Ok(meta) = fs::metadata(&path) {
                if let Ok(modified) = meta.modified() {
                    if let Ok(dur) = now.duration_since(modified) {
                        if dur.as_secs() > week_secs {
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    // 0. Check if this was launched via tck:// protocol (second instance).
    //    If so and another instance is already running, focus its window.
    let args: Vec<String> = std::env::args().collect();
    let is_tck_launch = args.iter().any(|a| a.starts_with("tck://"));

    // 1. Single-instance lock.
    if let Err(msg) = acquire_single_instance_lock() {
        eprintln!("{msg}");
        if is_tck_launch {
            // When launched via tck:// protocol, just activate the existing
            // window instead of showing a "duplicate" dialog.
            focus_existing_window();
        } else {
            show_duplicate_instance_dialog();
        }
        std::process::exit(0);
    }

    // 1b. Register the tck:// protocol handler on first launch.
    register_protocol_handler();

    // 2. Extract embedded WASM assets to a temp directory.
    let _tmp = extract_assets();
    let dist_dir = _tmp.path().to_str().unwrap().to_string();

    // 3. Build the event loop (typed so we can route custom events).
    let event_loop = EventLoopBuilder::<DesktopEvent>::new().build();
    let proxy = event_loop.create_proxy();

    // 4. Background thread: poll for focus trigger file (written by a second
    //    instance launched via tck:// protocol) + periodically allow any
    //    process to set foreground (Windows).
    thread::spawn(move || {
        let trigger = std::env::temp_dir().join("tck-desktop.focus");
        loop {
            if trigger.exists() {
                let _ = fs::remove_file(&trigger);
                let _ = proxy.send_event(DesktopEvent::FocusWindow);
            }

            #[cfg(target_os = "windows")]
            // SAFETY: AllowSetForegroundWindow with ASFW_ANY is safe to
            // call from any thread — it just sets a per-process flag.
            unsafe {
                extern "system" {
                    fn AllowSetForegroundWindow(dwProcessId: u32) -> i32;
                }
                // ASFW_ANY = -1 as u32 → any process can bring our window
                // to the foreground, fixing SetForegroundWindow denial for
                // second instances launched via tck:// protocol.
                AllowSetForegroundWindow(-1i32 as u32);
            }

            thread::sleep(Duration::from_secs(2));
        }
    });

    // 5. Start the tokio runtime + server on a background thread.
    //    Bind to port 0 so the OS assigns a free port.
    let (addr_tx, addr_rx) = mpsc::channel();
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        rt.block_on(async {
            match tck_server::serve_on_addr("127.0.0.1:0", &dist_dir).await {
                Ok((addr, _event_rx)) => {
                    let _ = addr_tx.send(addr);
                }
                Err(_) => {
                    eprintln!("T.C.K server failed to start");
                    std::process::exit(1);
                }
            }
            // Keep runtime alive so the server keeps running.
            loop {
                tokio::time::sleep(Duration::from_secs(3600)).await;
            }
        });
    });

    // 6. Wait for the server to be ready.
    let server_addr: std::net::SocketAddr = addr_rx.recv().expect("failed to get server address");
    let addr_str = server_addr.to_string();
    // Append #app so the UI skips the landing page inside the desktop app.
    let url = format!("http://{server_addr}/#app");

    // 7. Create the main window and webview.
    let window = WindowBuilder::new()
        .with_title("T.C.K - Multi-Instance Terminal Manager")
        .with_inner_size(tao::dpi::LogicalSize::new(1200.0, 800.0))
        .build(&event_loop)
        .expect("failed to create window");

    let _webview = WebViewBuilder::new()
        .with_url(&url)
        .build(&window)
        .expect("failed to create webview");

    // 8. Build the system tray icon.
    use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tray_icon::Icon;

    let show_hide_item = MenuItem::with_id("show_hide", "Show/Hide T.C.K", true, None);
    let show_hide_id = show_hide_item.id().clone();
    let quit_item = MenuItem::with_id("quit", "Quit", true, None);
    let quit_id = quit_item.id().clone();

    let tray_menu = Menu::new();
    let _ = tray_menu.append(&show_hide_item);
    let _ = tray_menu.append(&PredefinedMenuItem::separator());
    let _ = tray_menu.append(&quit_item);

    let rgba = blue_circle_rgba();
    let icon = Icon::from_rgba(rgba, 32, 32).expect("failed to create tray icon from RGBA");

    let _tray_icon = TrayIconBuilder::new()
        .with_icon(icon)
        .with_menu(Box::new(tray_menu))
        .with_tooltip("T.C.K — Multi-Instance Terminal Manager")
        .build()
        .expect("failed to build tray icon");

    // 9. Main event loop.
    let addr_for_save: String = addr_str;

    // Clean up old sessions at startup (runs once).
    clean_old_sessions();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // ── User events (from protocol-handler signaling thread) ──
        if let tao::event::Event::UserEvent(DesktopEvent::FocusWindow) = &event {
            window.set_visible(true);
            window.focus();
        }

        // ── Window events ──────────────────────────────────────────────
        if let tao::event::Event::WindowEvent {
            event: tao::event::WindowEvent::CloseRequested,
            ..
        } = event
        {
            save_session(&addr_for_save);
            *control_flow = ControlFlow::Exit;
            return;
        }

        // ── Tray icon events ──────────────────────────────────────────
        if let Ok(tray_event) = tray_icon::TrayIconEvent::receiver().try_recv() {
            if matches!(
                tray_event,
                tray_icon::TrayIconEvent::Click {
                    button: tray_icon::MouseButton::Left,
                    ..
                }
            ) {
                let visible = window.is_visible();
                window.set_visible(!visible);
            }
        }

        // ── Menu events ───────────────────────────────────────────────
        if let Ok(menu_event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
            if menu_event.id() == &show_hide_id {
                let visible = window.is_visible();
                window.set_visible(!visible);
            } else if menu_event.id() == &quit_id {
                save_session(&addr_for_save);
                *control_flow = ControlFlow::Exit;
            }
        }
    });
}
