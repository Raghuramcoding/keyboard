@echo off
title Claude Code
echo ============================================
echo   Claude Code v2.0 - Starting...
echo ============================================
echo.

cd /d "%~dp0"

:: Check Node.js (the only requirement for running from source)
node --version >nul 2>&1
if %errorlevel% neq 0 (
    echo ERROR: Node.js is not installed or not in PATH.
    echo Please install Node.js 18+ from https://nodejs.org
    pause
    exit /b 1
)

:: Install dependencies if needed
if not exist "node_modules\" (
    echo [1/2] Installing npm dependencies...
    call npm install
) else (
    echo [1/2] Dependencies ready
)

:: Build
echo [2/2] Building...
call npm run build

echo.
echo ============================================
echo   Launching Claude Code...
echo ============================================
npm start
