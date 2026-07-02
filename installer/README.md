# T.C.K Installers & Packages

This directory contains scripts and configuration for building distributable
T.C.K installers and packages.

## Windows Installer (Inno Setup)

### Prerequisites

1. [Inno Setup](https://jrsoftware.org/isdl.php) (or just `ISCC.exe` on PATH)
2. **Visual Studio Build Tools** with the MSVC compiler (for compiling the
   Rust server binary on Windows)
3. Rust, Trunk (see main README)

### Build an installer

```powershell
# 1. Build the WASM UI and server
.\scripts\build.ps1

# 2. Compile the installer
ISCC.exe installer\windows\setup.iss
# Creates: dist\TCK-Setup-3.0.0.exe
```

Double-click the generated `TCK-Setup-3.0.0.exe` to install.

The installer places:
- `tck-server.exe` — the native server binary
- `dist/` — the WASM bundle (served by the server)
- `tck-launcher.ps1` — PowerShell script that starts the server & opens the
  browser

Start menu shortcut runs the launcher, which starts the server and opens
`http://127.0.0.1:3000` in your default browser.

### Uninstall

Via **Windows Settings → Apps → Installed apps → T.C.K**, or the Start menu
Uninstall shortcut.

## Desktop App (Native Webview)

For a native window (no browser chrome), the `tck-desktop` crate embeds the
server and opens a WebView2 window:

```powershell
.\scripts\build.ps1 -Desktop
cargo run -p tck-desktop
```

This produces `tck-desktop.exe` — a single binary that bundles everything
including the WASM assets. No separate server or browser needed.

**Note:** `tck-desktop` requires the MSVC linker (`link.exe`) from Visual Studio
Build Tools on Windows. See [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/).

## Build from Source (Portable)

If you don't want an installer, just compile and run:

```bash
cargo build -p tck-server --release
cargo run -p tck-server --release
# Open http://127.0.0.1:3000
```

The server serves the WASM UI from `crates/tck-ui/dist/` by default.
