//! Purpose: Where a server's programs are — the `PATH` directories, the
//! execute bit, and each platform's spelling of an executable's name.

use std::path::{Path, PathBuf};

pub(in crate::lsp::install) fn find_all(
    names: &[&str],
    paths: impl Iterator<Item = PathBuf>,
) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for directory in paths.filter(|path| path.is_absolute()) {
        for name in names {
            let path = directory.join(executable(name));
            if runnable(&path) {
                found.push(path);
            }
        }
    }
    found
}

#[cfg(unix)]
pub(in crate::lsp::install) fn runnable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(windows)]
pub(in crate::lsp::install) fn runnable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(windows)]
pub fn executable(name: &str) -> String {
    if Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        name.to_owned()
    } else {
        format!("{name}.exe")
    }
}
#[cfg(unix)]
pub fn executable(name: &str) -> String {
    name.to_owned()
}

/// Whether a program found on `PATH` under `name` is one LSP lookup would
/// start. Windows lookup appends `.exe` ([`executable`]), so only an `.exe`
/// counts there — not a `.dll`, a `.mui` or npm's extensionless shim. Unix
/// lookup takes the name as it is, and the `PATH` scan already required the
/// execute bit.
#[cfg(windows)]
pub fn launchable_name(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
}
#[cfg(unix)]
pub const fn launchable_name(_name: &str) -> bool {
    true
}
#[cfg(windows)]
pub(in crate::lsp::install) fn venv_python(root: &Path) -> PathBuf {
    root.join("venv/Scripts/python.exe")
}
#[cfg(unix)]
pub(in crate::lsp::install) fn venv_python(root: &Path) -> PathBuf {
    root.join("venv/bin/python")
}
