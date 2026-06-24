Write-Host "Starting Claude Code..."
Push-Location -Path $PSScriptRoot
if (-not (Test-Path "node_modules")) { npm install }
npm run build
npm start
Pop-Location
