//! Read real paths and filesystem short names for command substitution.

use std::path::{Component, Path, PathBuf};

use crate::models::apply::{Context, Names, PanelContext, Paths};
use crate::models::{App, Panel, PanelSide};

pub(super) fn capture(app: &App, current: &str) -> Context {
    Context {
        left: panel(
            &app.left_panel,
            (app.active_panel == PanelSide::Left).then_some(current),
        ),
        right: panel(
            &app.right_panel,
            (app.active_panel == PanelSide::Right).then_some(current),
        ),
        active: app.active_panel,
    }
}

/// Reuse the captured aliases throughout the batch. Looking up every marked
/// file on disk for every command would make a plain bulk command quadratic
/// in filesystem requests, even when it never uses a selection substitution.
pub(super) fn select(context: &mut Context, app: &App, name: &str) {
    let panel = match context.active {
        PanelSide::Left => &app.left_panel,
        PanelSide::Right => &app.right_panel,
    };
    let target = context.panel_mut(context.active);
    target.entry = target
        .selected
        .iter()
        .find(|entry| entry.long == name)
        .cloned()
        .unwrap_or_else(|| entry_names(&panel.path, name));
    target.description = panel
        .entries
        .iter()
        .find(|entry| entry.name == name)
        .and_then(|entry| entry.description.clone())
        .unwrap_or_default();
}

fn panel(panel: &Panel, current: Option<&str>) -> PanelContext {
    let current = current
        .or_else(|| panel.get_selected_entry().map(|entry| entry.name.as_str()))
        .unwrap_or("");
    let resolved = panel
        .path
        .canonicalize()
        .unwrap_or_else(|_| panel.path.clone());
    PanelContext {
        paths: Paths {
            logical: path_names(&panel.path),
            resolved: path_names(&resolved),
            drive: drive(&panel.path),
        },
        entry: entry_names(&panel.path, current),
        selected: selected_names(panel)
            .iter()
            .map(|name| entry_names(&panel.path, name))
            .collect(),
        description: panel
            .entries
            .iter()
            .find(|entry| entry.name == current)
            .and_then(|entry| entry.description.clone())
            .unwrap_or_default(),
    }
}

pub(super) fn selected_names(panel: &Panel) -> Vec<String> {
    if panel.marked.is_empty() {
        return panel.action_entry_names();
    }
    panel
        .entries
        .iter()
        .filter(|entry| entry.name != ".." && panel.marked.contains(entry.name.as_str()))
        .map(|entry| entry.name.clone())
        .collect()
}

fn entry_names(directory: &Path, name: &str) -> Names {
    let short = short_path(&directory.join(name));
    Names {
        long: name.to_owned(),
        short: short.file_name().map_or_else(
            || name.to_owned(),
            |name| name.to_string_lossy().into_owned(),
        ),
    }
}

fn path_names(path: &Path) -> Names {
    Names {
        long: display_path(path),
        short: display_path(&short_path(path)),
    }
}

pub(super) fn display_path(path: &Path) -> String {
    crate::path_display::without_verbatim_prefix(&path.to_string_lossy()).into_owned()
}

fn drive(path: &Path) -> String {
    match path.components().next() {
        Some(Component::Prefix(prefix)) => display_path(Path::new(prefix.as_os_str())),
        Some(Component::RootDir) => std::path::MAIN_SEPARATOR.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests;

#[cfg(windows)]
#[allow(unsafe_code, reason = "GetShortPathNameW has no safe wrapper in std")]
pub(super) fn short_path(path: &Path) -> PathBuf {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;

    let source: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: source is NUL-terminated; a null output requests the buffer size.
    let size = unsafe { GetShortPathNameW(source.as_ptr(), std::ptr::null_mut(), 0) };
    if size == 0 {
        return path.to_owned();
    }
    let Ok(capacity) = usize::try_from(size) else {
        return path.to_owned();
    };
    let mut buffer = vec![0; capacity];
    // SAFETY: the writable buffer has the exact capacity passed to Windows.
    let written = unsafe { GetShortPathNameW(source.as_ptr(), buffer.as_mut_ptr(), size) };
    if written == 0 || written >= size {
        return path.to_owned();
    }
    let Ok(length) = usize::try_from(written) else {
        return path.to_owned();
    };
    buffer.truncate(length);
    PathBuf::from(std::ffi::OsString::from_wide(&buffer))
}

#[cfg(not(windows))]
pub(super) fn short_path(path: &Path) -> PathBuf {
    path.to_owned()
}

/// Some DOS tools recreate a file under its 8.3 alias. `!+!` restores the
/// original long name if it disappeared; an existing long name is untouched.
pub(super) fn restore_long_name(directory: &Path, name: &Names) -> std::io::Result<()> {
    let long = directory.join(&name.long);
    let short = directory.join(&name.short);
    if name.long != name.short && !long.try_exists()? && short.try_exists()? {
        std::fs::rename(short, long)?;
    }
    Ok(())
}
