//! Purpose: Fixture programs read private PTY input and remain cancellable without system changes.

use std::path::Path;

#[cfg(windows)]
pub(super) fn command(root: &Path) -> String {
    let path = root.join("private-shell-fixture.cmd");
    std::fs::write(&path, WINDOWS_SCRIPT).unwrap();
    format!("\"{}\"", path.display())
}

#[cfg(windows)]
const WINDOWS_SCRIPT: &str = "@echo off\r\nsetlocal EnableDelayedExpansion\r\necho ffm-ready\r\nset /p line=\r\necho reply:!line!\r\nping.exe -n 30 127.0.0.1 >nul\r\necho ffm-e2e-after\r\n";

#[cfg(not(windows))]
pub(super) fn command(_root: &Path) -> String {
    "printf 'ffm-ready\\n'; IFS= read -r line; printf 'reply:%s\\n' \"$line\"; sleep 30; printf 'ffm-e2e-after\\n'".to_owned()
}
