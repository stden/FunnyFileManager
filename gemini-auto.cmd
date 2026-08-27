@REM Purpose: Starts the Gemini automation loop on Windows.
@echo off
setlocal

cd /d "%~dp0"

set "PROMPT=%*"
if "%PROMPT%"=="" set "PROMPT=Inspect this FunnyFileManager repository, implement all required missing functions, run cargo build and cargo test --features test-helpers, fix any failures, and stop only when the project is in a working state."

where gemini >nul 2>nul
if errorlevel 1 (
    echo Gemini CLI was not found in PATH.
    exit /b 1
)

gemini -p "%PROMPT%"
exit /b %errorlevel%
