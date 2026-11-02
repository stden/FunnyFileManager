//! Purpose: Fixture programs read private PTY input and remain cancellable without system changes.

use std::path::Path;

#[cfg(windows)]
pub(super) fn command(root: &Path) -> String {
    let path = root.join("private-shell-fixture.ps1");
    std::fs::write(&path, WINDOWS_SCRIPT).unwrap();
    format!(
        "powershell.exe -NoLogo -NoProfile -NonInteractive -File \"{}\"",
        path.display()
    )
}

#[cfg(windows)]
const WINDOWS_SCRIPT: &str = "Write-Output 'ffm-ready'\n$line=[Console]::ReadLine()\nWrite-Output ('reply:'+$line)\nStart-Sleep -Seconds 30\nWrite-Output 'ffm-e2e-after'\n";

#[cfg(not(windows))]
pub(super) fn command(_root: &Path) -> String {
    "printf 'ffm-ready\\n'; IFS= read -r line; printf 'reply:%s\\n' \"$line\"; sleep 30; printf 'ffm-e2e-after\\n'".to_owned()
}
