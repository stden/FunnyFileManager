//! Purpose: the `.kdbx` trigger shared by `Enter` (open) and `F3` (view).
//!
//! One helper, two call sites. The recurring bug in this codebase is a rule
//! applied at one call site and quietly not at its twin, so the extension test,
//! the directory guard and the overlay construction all live here and both
//! actions call the same function — there is nothing left to keep in step.

use std::io;
use std::path::Path;

use crate::models::App;

/// Open the vault browser for `path` when it is a `KeePass` database.
///
/// Returns whether the overlay was opened, so the caller can skip its normal
/// behaviour for that key.
#[cfg(feature = "keepass")]
pub fn open_vault_if_kdbx(app: &mut App, path: &Path) -> bool {
    // A *directory* named `secrets.kdbx` is still a directory: it must keep
    // being entered by Enter and refused by F3, not handed to a parser that
    // will only fail on it.
    if path.is_dir() || !is_vault_file(path) {
        return false;
    }
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned();
    app.set_overlay(crate::models::Overlay::Kdbx(crate::models::KdbxOverlay(
        crate::kdbx::KdbxSession::new(path.to_path_buf()),
    )));
    app.set_status(crate::status::Status::VaultOpened(name));
    true
}

/// Without the `keepass` feature there is no vault browser, so every file keeps
/// its ordinary open / view behaviour.
#[cfg(not(feature = "keepass"))]
pub fn open_vault_if_kdbx(_app: &mut App, _path: &Path) -> bool {
    false
}

/// Extensions that open the vault browser.
///
/// `kbdx` is the common typo/transposition of `kdbx` and is accepted because
/// users really do name files that way; both are matched case-insensitively, so
/// `Vault.KDBX` from a Windows share opens like any other.
#[cfg(feature = "keepass")]
const VAULT_EXTENSIONS: [&str; 2] = ["kdbx", "kbdx"];

/// Whether `path`'s extension names a `KeePass` database.
#[cfg(feature = "keepass")]
fn is_vault_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            VAULT_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

/// `Enter` on the active panel's selection.
///
/// A `.kdbx` file opens the vault browser; everything else keeps the panel's
/// own behaviour — descend into a directory, do nothing for a plain file.
pub fn handle_open(app: &mut App) -> io::Result<()> {
    if let Some(selected) = app.get_active_panel().selected_path() {
        if open_vault_if_kdbx(app, &selected) {
            return Ok(());
        }
    }
    app.get_active_panel().enter()
}

#[cfg(all(test, feature = "keepass"))]
mod tests;
