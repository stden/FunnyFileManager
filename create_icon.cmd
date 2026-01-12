@echo off
:: Create Desktop Icon for FunnyFileManager on Windows
:: This script creates a desktop shortcut for FFM

setlocal enabledelayedexpansion

echo [FFM] Creating desktop shortcut for FunnyFileManager...

:: Get current directory
set "SCRIPT_DIR=%~dp0"
set "SCRIPT_DIR=%SCRIPT_DIR:~0,-1%"

:: Get desktop path
for /f "tokens=3*" %%a in ('reg query "HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders" /v Desktop') do set "DESKTOP=%%b"

:: FFM executable path
set "FFM_EXE=%SCRIPT_DIR%\target\release\ffm.exe"

:: Check if FFM is built
if not exist "%FFM_EXE%" (
    echo [WARNING] FFM executable not found at %FFM_EXE%
    echo [INFO] Building FFM in release mode...
    
    cd /d "%SCRIPT_DIR%"
    call cargo build --release
    
    if not exist "%FFM_EXE%" (
        echo [ERROR] Failed to build FFM. Please run 'cargo build --release' manually.
        pause
        exit /b 1
    )
)

:: Create VBS script to create shortcut
set "VBS_FILE=%TEMP%\create_ffm_shortcut.vbs"
(
echo Set oWS = WScript.CreateObject("WScript.Shell"^)
echo Set oLink = oWS.CreateShortcut("%DESKTOP%\FunnyFileManager.lnk"^)
echo.
echo ' Check if Windows Terminal exists
echo On Error Resume Next
echo Dim wtPath
echo wtPath = oWS.ExpandEnvironmentStrings("%%LOCALAPPDATA%%\Microsoft\WindowsApps\wt.exe"^)
echo If oWS.Environment("Process"^).Item("WT_SESSION"^) ^<^> "" Or objFSO.FileExists(wtPath^) Then
echo     ' Use Windows Terminal if available
echo     oLink.TargetPath = "wt.exe"
echo     oLink.Arguments = "-d """ ^& "%SCRIPT_DIR%" ^& """ -- """ ^& "%FFM_EXE%" ^& """"
echo Else
echo     ' Use cmd.exe as fallback
echo     oLink.TargetPath = "cmd.exe"
echo     oLink.Arguments = "/k """ ^& "%FFM_EXE%" ^& """"
echo End If
echo On Error Goto 0
echo.
echo oLink.WorkingDirectory = "%SCRIPT_DIR%"
echo oLink.IconLocation = "%FFM_EXE%,0"
echo oLink.Description = "Funny File Manager - Terminal file manager inspired by Far Manager"
echo oLink.Save
) > "%VBS_FILE%"

:: Run VBS script
cscript //nologo "%VBS_FILE%"
if %errorlevel% equ 0 (
    echo [SUCCESS] Desktop shortcut created: %DESKTOP%\FunnyFileManager.lnk
) else (
    echo [ERROR] Failed to create shortcut
)

:: Clean up
del "%VBS_FILE%" 2>nul

:: Optional: Create Start Menu shortcut
set "START_MENU=%APPDATA%\Microsoft\Windows\Start Menu\Programs"
if exist "%START_MENU%" (
    set "VBS_FILE_START=%TEMP%\create_ffm_start.vbs"
    (
    echo Set oWS = WScript.CreateObject("WScript.Shell"^)
    echo Set oLink = oWS.CreateShortcut("%START_MENU%\FunnyFileManager.lnk"^)
    echo oLink.TargetPath = "%FFM_EXE%"
    echo oLink.WorkingDirectory = "%SCRIPT_DIR%"
    echo oLink.IconLocation = "%FFM_EXE%,0"
    echo oLink.Description = "Funny File Manager"
    echo oLink.Save
    ) > "%VBS_FILE_START%"
    
    cscript //nologo "%VBS_FILE_START%"
    if %errorlevel% equ 0 (
        echo [SUCCESS] Start Menu shortcut created
    )
    del "%VBS_FILE_START%" 2>nul
)

echo.
echo [INFO] Shortcuts created successfully!
echo [INFO] You can now launch FunnyFileManager from your desktop
echo.
pause