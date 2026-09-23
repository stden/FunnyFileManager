//! Purpose: Language tables for the command shells: POSIX shells, Windows batch and PowerShell.

use super::{LineMode, Syntax};

pub(super) const SHELL: Syntax = Syntax {
    mode: LineMode::Tokens,
    line_comments: &["#"],
    block_comment: None,
    quotes: &['"', '\''],
    keywords: &[
        "case", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if",
        "in", "local", "return", "select", "then", "until", "while",
    ],
    types: &[
        "echo", "cd", "exit", "read", "set", "source", "test", "unset",
    ],
    meta_prefix: None,
    section_headers: false,
    variable_prefix: Some('$'),
    word_hyphens: false,
    switch_prefix: None,
};

/// Windows batch. Far paints it sparely: numbers and switches stand out and
/// almost everything else stays plain, which is what a `.cmd` file mostly is.
pub(super) const BATCH: Syntax = Syntax {
    mode: LineMode::Tokens,
    line_comments: &["rem ", "REM ", "::"],
    block_comment: None,
    // Quoted text is not a string here. `set "NAME=value"` uses quotes for
    // grouping, not for literals, and colouring them as strings paints half of
    // every batch file one colour.
    quotes: &[],
    keywords: &[
        "call", "do", "echo", "else", "endlocal", "exist", "exit", "for", "goto", "if", "in",
        "not", "pause", "popd", "pushd", "set", "setlocal", "shift", "start",
    ],
    types: &["defined", "errorlevel", "nul"],
    meta_prefix: None,
    section_headers: false,
    switch_prefix: Some('/'),
    variable_prefix: None,
    word_hyphens: false,
};

/// PowerShell. Nothing like `cmd`, which is where `.ps1` used to land: it has
/// block comments, real strings, `$variables` everywhere and `-Parameters` on
/// every call, none of which a batch scheme has anything to say about.
pub(super) const POWERSHELL: Syntax = Syntax {
    mode: LineMode::Tokens,
    line_comments: &["#"],
    block_comment: Some(("<#", "#>")),
    quotes: &['"', '\''],
    keywords: &[
        "begin",
        "break",
        "catch",
        "class",
        "continue",
        "data",
        "do",
        "dynamicparam",
        "else",
        "elseif",
        "end",
        "enum",
        "exit",
        "filter",
        "finally",
        "for",
        "foreach",
        "from",
        "function",
        "hidden",
        "if",
        "in",
        "param",
        "process",
        "return",
        "static",
        "switch",
        "throw",
        "trap",
        "try",
        "until",
        "using",
        "while",
    ],
    // Cmdlets rather than types: PowerShell has no type keywords worth the
    // name, and the verb-noun calls are what a reader is actually scanning for.
    types: &[
        "Add-Content",
        "Compare-Object",
        "ConvertTo-Json",
        "ConvertFrom-Json",
        "Copy-Item",
        "Get-ChildItem",
        "Get-Content",
        "Get-Date",
        "Get-Item",
        "Get-Location",
        "Get-Member",
        "Get-Process",
        "Invoke-Expression",
        "Invoke-RestMethod",
        "Invoke-WebRequest",
        "Join-Path",
        "Measure-Object",
        "New-Item",
        "New-Object",
        "Out-File",
        "Out-Null",
        "Remove-Item",
        "Select-Object",
        "Set-Content",
        "Set-Location",
        "Sort-Object",
        "Start-Process",
        "Stop-Process",
        "Test-Path",
        "Where-Object",
        "Write-Error",
        "Write-Host",
        "Write-Output",
        "Write-Warning",
    ],
    meta_prefix: None,
    section_headers: false,
    // `-ExecutionPolicy`, and the operators too: `-lt`, `-and`, `-match` are
    // spelled the same way and read as the same kind of thing.
    switch_prefix: Some('-'),
    variable_prefix: Some('$'),
    word_hyphens: true,
};
