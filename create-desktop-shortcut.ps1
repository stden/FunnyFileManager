# Purpose: Creates a Windows desktop shortcut for launching FunnyFileManager.
# Create Desktop Shortcut for FunnyFileManager
# This script creates a shortcut on the desktop that launches FFM in Windows Terminal

$DesktopPath = [Environment]::GetFolderPath("Desktop")
$ShortcutPath = Join-Path $DesktopPath "FunnyFileManager.lnk"

# Path to the FFM executable
$FFMPath = Join-Path $PSScriptRoot "target\release\ffm.exe"

# Check if FFM is built
if (-not (Test-Path $FFMPath)) {
    Write-Host "FFM executable not found at $FFMPath" -ForegroundColor Red
    Write-Host "Building FFM in release mode..." -ForegroundColor Yellow
    
    Push-Location $PSScriptRoot
    cargo build --release
    Pop-Location
    
    if (-not (Test-Path $FFMPath)) {
        Write-Host "Failed to build FFM. Please run 'cargo build --release' manually." -ForegroundColor Red
        exit 1
    }
}

# Create the shortcut
$WScriptShell = New-Object -ComObject WScript.Shell
$Shortcut = $WScriptShell.CreateShortcut($ShortcutPath)

# Check if Windows Terminal is available
$WindowsTerminal = Get-Command wt.exe -ErrorAction SilentlyContinue

if ($WindowsTerminal) {
    # Use Windows Terminal
    $Shortcut.TargetPath = "wt.exe"
    $Shortcut.Arguments = "-d `"$PSScriptRoot`" -- `"$FFMPath`""
} else {
    # Use FFM directly
    $Shortcut.TargetPath = $FFMPath
    $Shortcut.WorkingDirectory = $PSScriptRoot
}

# Set icon (you can add a custom .ico file later)
$Shortcut.IconLocation = "$FFMPath,0"

# Set description
$Shortcut.Description = "Funny File Manager - Terminal-based file manager inspired by Far Manager"

# Save the shortcut
$Shortcut.Save()

Write-Host "Desktop shortcut created successfully at: $ShortcutPath" -ForegroundColor Green

# Optional: Create a batch file alternative
$BatchPath = Join-Path $DesktopPath "FunnyFileManager.bat"
$BatchContent = @"
@echo off
cd /d "$PSScriptRoot"
start "" "$FFMPath"
"@

Set-Content -Path $BatchPath -Value $BatchContent
Write-Host "Batch file created at: $BatchPath" -ForegroundColor Green