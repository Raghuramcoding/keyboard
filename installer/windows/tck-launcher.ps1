# T.C.K — TalentCloud Keyboard: Launcher
# =======================================
# Starts the tck-server and opens the browser.
# Press Ctrl+C in this terminal to stop.

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $PSCommandPath

# The server binary lives next to this script, or in the same directory.
$ServerExe = Join-Path $ScriptDir "tck-server.exe"
if (-not (Test-Path $ServerExe)) {
    # Try the parent directory (portable layout).
    $ServerExe = Join-Path (Split-Path -Parent $ScriptDir) "tck-server.exe"
}
if (-not (Test-Path $ServerExe)) {
    Write-Error "tck-server.exe not found next to this script."
    exit 1
}

$Addr = "127.0.0.1:3000"
$Proc = Start-Process -PassThru -NoNewWindow -FilePath $ServerExe -ArgumentList @()

Write-Host "T.C.K server started on http://$Addr" -ForegroundColor Green
Start-Sleep -Seconds 1
Start-Process "http://127.0.0.1:3000"

Write-Host "Press Ctrl+C to stop the server." -ForegroundColor Yellow

# Wait for the process to exit or for Ctrl+C.
try {
    $Proc.WaitForExit()
} catch {
    $Proc.Kill()
}
