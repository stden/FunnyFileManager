//! Purpose: the `.kdbx` trigger shared by `Enter` (open) and `F3` (view).
//!
//! One helper, two call sites. The recurring bug in this codebase is a rule
//! applied at one call site and quietly not at its twin, so the extension test,
//! the directory guard and the overlay construction all live here and both
//! actions call the same function — there is nothing left to keep in step.

use crate::models::App;

/// Open the vault browser when the active panel's selection is a `KeePass`
/// database.
///
/// Returns whether the overlay was opened, so the caller can skip its normal
/// behaviour for that key.
#[cfg(feature = "keepass")]
pub fn open_vault_if_kdbx(app: &mut App) -> bool {
    open(app, crate::kdbx::Access::View)
}

#[cfg(feature = "keepass")]
fn open(app: &mut App, access: crate::kdbx::Access) -> bool {
    let Some(path) = app.get_active_panel().selected_path() else {
        return false;
    };
    // A *directory* named `secrets.kdbx` is still a directory: it must keep
    // being entered by Enter and refused by F3, not handed to a parser that
    // will only fail on it.
    if path.is_dir() || !is_vault_file(&path) {
        return false;
    }
    let name = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned();
    let path = std::fs::canonicalize(&path).unwrap_or(path);
    let session = app
        .vault_cache
        .take(path, access, std::time::Instant::now());
    app.set_overlay(crate::models::Overlay::Kdbx(crate::models::KdbxOverlay(
        session,
    )));
    app.set_status(crate::status::Status::VaultOpened(name));
    true
}

pub fn edit_vault_if_kdbx(app: &mut App) -> bool {
    #[cfg(feature = "keepass")]
    {
        open(app, crate::kdbx::Access::Edit)
    }
    #[cfg(not(feature = "keepass"))]
    {
        let _ = app;
        false
    }
}

/// Without the `keepass` feature there is no vault browser, so every file keeps
/// its ordinary open / view behaviour.
#[cfg(not(feature = "keepass"))]
pub fn open_vault_if_kdbx(_app: &mut App) -> bool {
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
fn is_vault_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            VAULT_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

#[cfg(all(test, feature = "keepass"))]
mod tests;
#[cfg(all(test, feature = "keepass"))]
mod tests_maintenance;

pub fn maintenance(app: &mut App) -> bool {
    #[cfg(feature = "keepass")]
    {
        maintain_secrets(app, std::time::Instant::now())
    }
    #[cfg(not(feature = "keepass"))]
    {
        let _ = app;
        false
    }
}

#[cfg(feature = "keepass")]
fn maintain_secrets(app: &mut App, now: std::time::Instant) -> bool {
    app.vault_cache.expire(now);
    let locked = app
        .kdbx_session_mut()
        .is_some_and(|session| session.tick(now));
    if locked {
        app.set_status(crate::status::Status::VaultLocked);
    }
    if app.clipboard.take_if_due(now) {
        if let Some(backend) = app.clipboard.backend_mut() {
            let _ = crate::clipboard::clear(backend);
        }
    }
    locked
}
