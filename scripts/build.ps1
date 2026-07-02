# T.C.K — TalentCloud Keyboard: Build & Installer Script
# ========================================================
# Prerequisites: Rust, Trunk, PowerShell 7+
#
#   .\scripts\build.ps1          # build everything (release)
#   .\scripts\build.ps1 -Dev     # build dev profile
#   .\scripts\build.ps1 -Desktop # include desktop app (requires MSVC)

param(
    [switch]$Dev,
    [switch]$Desktop
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$Profile = if ($Dev) { "dev" } else { "release" }
$ProfileFlag = if ($Dev) { "" } else { "--release" }

Write-Host "=== T.C.K Build Script ===" -ForegroundColor Cyan
Write-Host "Profile: $Profile"

# ---- 1. Build the WASM UI ----
Write-Host "`n[1/3] Building WASM UI (tck-ui)..." -ForegroundColor Green
Push-Location "$Root\crates\tck-ui"
try {
    trunk build @($ProfileFlag -split ' ')
    Write-Host "  => crates/tck-ui/dist/" -ForegroundColor Green
} finally {
    Pop-Location
}

# ---- 2. Build the native server ----
Write-Host "`n[2/3] Building native server (tck-server)..." -ForegroundColor Green
Push-Location $Root
try {
    cargo build -p tck-server @($ProfileFlag -split ' ')
    $BuildDir = if ($Dev) { "debug" } else { "release" }
    Write-Host "  => target/$BuildDir/tck-server.exe" -ForegroundColor Green
} finally {
    Pop-Location
}

# ---- 3. Desktop app (optional) ----
if ($Desktop) {
    Write-Host "`n[3/3] Building desktop app (tck-desktop)..." -ForegroundColor Green
    Push-Location $Root
    try {
        cargo build -p tck-desktop @($ProfileFlag -split ' ')
        Write-Host "  => target/$BuildDir/tck-desktop.exe" -ForegroundColor Green
    } finally {
        Pop-Location
    }
} else {
    Write-Host "`n[3/3] Skipped (use -Desktop to include the native webview app)" -ForegroundColor Yellow
}

# ---- Package summary ----
Write-Host "`n=== Build complete ===" -ForegroundColor Cyan
Write-Host "Server binary: target/$Profile/tck-server.exe"
Write-Host "Web assets:    crates/tck-ui/dist/"
Write-Host "`nTo run: tck-server.exe (serves on http://127.0.0.1:3000)"
