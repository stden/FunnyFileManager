#!/usr/bin/env pwsh
param(
    [string]$Destination = "docs/upstream/far-manager",
    [string]$RepoUrl = "https://github.com/FarGroup/FarManager.git",
    [switch]$Clean
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "git is required (missing from PATH)."
}

if ($Clean -and (Test-Path $Destination)) {
    Remove-Item -Recurse -Force $Destination
}

New-Item -ItemType Directory -Force -Path $Destination | Out-Null

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("ffm-farmanager-docs-" + [guid]::NewGuid().ToString("n"))
$tempRepo = Join-Path $tempRoot "FarManager"

try {
    New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null
    git clone --depth 1 $RepoUrl $tempRepo | Out-Host

    $commit = (git -C $tempRepo rev-parse HEAD).Trim()
    $origin = (git -C $tempRepo config --get remote.origin.url).Trim()

    $repoRoot = (Resolve-Path $tempRepo).Path.TrimEnd("\")
    $destRoot = (Resolve-Path $Destination).Path.TrimEnd("\")

    $docExtensions = @(
        ".md", ".markdown", ".rst", ".txt",
        ".hlf", ".chm",
        ".html", ".htm", ".pdf",
        ".png", ".jpg", ".jpeg", ".gif", ".svg"
    )

    $docFiles =
        Get-ChildItem -Path $repoRoot -Recurse -File -Force |
        Where-Object { $_.FullName -notmatch "\\\\.git\\\\" } |
        Where-Object {
            $ext = $_.Extension.ToLowerInvariant()
            ($docExtensions -contains $ext) -or ($_.Name -match "^(README|LICENSE|COPYING)(\\..*)?$")
        }

    foreach ($file in $docFiles) {
        $relative = $file.FullName.Substring($repoRoot.Length + 1)
        $destPath = Join-Path $destRoot $relative
        $destDir = Split-Path -Parent $destPath
        New-Item -ItemType Directory -Force -Path $destDir | Out-Null
        Copy-Item -Force -Path $file.FullName -Destination $destPath
    }

    $source = [ordered]@{
        repo = $origin
        commit = $commit
        fetched_at_utc = [DateTime]::UtcNow.ToString("o")
        file_count = @($docFiles).Count
    }
    ($source | ConvertTo-Json -Depth 4) + "`n" | Set-Content -Encoding UTF8 (Join-Path $destRoot "SOURCE.json")

    Write-Host "Downloaded $(@($docFiles).Count) documentation files to '$Destination' (commit $commit)."
}
finally {
    if (Test-Path $tempRoot) {
        Remove-Item -Recurse -Force $tempRoot
    }
}
