//! Purpose: Disk Cleanup operation — scan candidate sizes, open the dialog,
//! and remove the selected entries when the user confirms.
//!
//! Entry point: `:cleanup`

use std::fs;
use std::io;
use std::path::Path;

use crate::models::{App, CleanupField, DiskCleanupDialog, FileSize, Overlay};
use crate::status::Status;
use crate::utils::refresh_panels;

/// `:cleanup` — measure the default candidates and open the checklist dialog.
pub fn open(app: &mut App) -> io::Result<()> {
    let mut dialog = DiskCleanupDialog::new();
    // Resolve relative paths (the Rust `target` dir) against the active panel.
    let panel_path = app.get_active_panel_ref().path.clone();
    measure(&mut dialog, &panel_path);
    app.set_overlay(Overlay::DiskCleanup(Box::new(dialog)));
    Ok(())
}

/// Cancel button / Esc: close without touching anything.
pub fn cancel(app: &mut App) {
    app.close_overlay();
    app.set_status(Status::CommandOutput("Cleanup cancelled".to_owned()));
}

/// "Clean up" button: delete every checked, existing candidate, then report.
pub fn run(app: &mut App) {
    let Overlay::DiskCleanup(ref dialog) = app.overlay else {
        return;
    };
    let paths = dialog.selected_paths();
    let total_selected = dialog.selected_bytes();
    app.close_overlay();

    if paths.is_empty() {
        app.set_status(Status::CommandOutput(
            "Nothing selected for cleanup".to_owned(),
        ));
        return;
    }

    let mut freed: u64 = 0;
    let mut errors: usize = 0;
    for path in &paths {
        match delete_path(path) {
            Ok(bytes) => freed += bytes,
            Err(_) => errors += 1,
        }
    }

    // Some files may have been in-use (locked by other processes) — report
    // what was freed rather than claiming the full selected amount.
    let _ = total_selected; // size estimate; actual freed may differ
    let size = FileSize::from_bytes(freed);
    let msg = if errors == 0 {
        format!("Freed {size}")
    } else {
        format!("Freed {size}, {errors} item(s) could not be removed")
    };
    app.set_status(Status::CommandOutput(msg));
    let _ = refresh_panels(app);
}

// ── internal helpers ──────────────────────────────────────────────────────────

/// Fill in `exists` and `size` for every candidate in `dialog`.
///
/// Relative paths are resolved against `base` (the active panel directory).
/// The JVM heap-dump candidate scans for `*.hprof` files instead of sizing
/// the whole home directory.
pub fn measure(dialog: &mut DiskCleanupDialog, base: &Path) {
    for candidate in &mut dialog.candidates {
        let abs = if candidate.path.is_absolute() {
            candidate.path.clone()
        } else {
            base.join(&candidate.path)
        };

        // The heap-dump candidate is special: `path` points at the home dir
        // but we only want the *.hprof files inside it, not the whole tree.
        if candidate.label.contains(".hprof") {
            let (size, exists) = measure_hprof(&abs);
            candidate.path = abs; // store resolved absolute path
            candidate.exists = exists;
            candidate.size = exists.then_some(size);
            candidate.checked = exists; // pre-tick if anything found
        } else {
            candidate.path.clone_from(&abs);
            candidate.exists = abs.exists();
            if candidate.exists {
                let stats = crate::stats::analyse(&abs);
                candidate.size = Some(stats.total);
                // Pre-tick the Temp folder and any existing hprof; leave
                // others unchecked so the user consciously opts in.
                candidate.checked = candidate.label.contains("Temp");
            }
        }
    }
}

/// Sum the sizes of `*.hprof` files directly inside `dir`.
///
/// Returns `(total_size, found_any)`.
fn measure_hprof(dir: &Path) -> (FileSize, bool) {
    let Ok(entries) = fs::read_dir(dir) else {
        return (FileSize::from_bytes(0), false);
    };
    let mut total: u64 = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().ends_with(".hprof") {
            if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    (FileSize::from_bytes(total), total > 0)
}

/// Delete one path: a directory recursively, a file directly.
///
/// Returns the bytes freed (best-effort: the size measured *before* deletion).
fn delete_path(path: &Path) -> io::Result<u64> {
    // For the heap-dump candidate `path` is the home dir — delete only *.hprof.
    if path.is_dir() && is_hprof_candidate(path) {
        return delete_hprof_files(path);
    }

    let size_before = size_of(path);
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(size_before)
}

/// True when `path` is a home directory that was picked as the hprof candidate.
/// We detect it heuristically: it is a directory that contains no `.gradle`
/// or `AppData` inside it (those would be cache dirs, not home dirs) — but the
/// simplest and most reliable signal is that it is not a path ending in a
/// cache-specific leaf like "caches", "wrapper", "_cacache", etc.
fn is_hprof_candidate(path: &Path) -> bool {
    path.file_name().is_some_and(|n| {
        let s = n.to_string_lossy();
        // Home dir names look like user names, not cache leaf names.
        !matches!(
            s.as_ref(),
            "caches"
                | "wrapper"
                | "_cacache"
                | "JetBrains"
                | "wsl"
                | "ms-playwright"
                | "pip"
                | "Cache"
                | "target"
        )
    })
}

fn delete_hprof_files(dir: &Path) -> io::Result<u64> {
    let mut freed: u64 = 0;
    let entries = fs::read_dir(dir)?;
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().ends_with(".hprof") {
            if let Ok(meta) = entry.metadata() {
                freed += meta.len();
            }
            let _ = fs::remove_file(entry.path()); // ignore per-file errors
        }
    }
    Ok(freed)
}

fn size_of(path: &Path) -> u64 {
    path.metadata().map_or(0, |m| {
        if m.is_dir() {
            crate::stats::analyse(path).total.bytes()
        } else {
            m.len()
        }
    })
}

/// Move the cursor forward (down / Tab) or backward (up / Shift+Tab).
pub fn advance_field(dialog: &mut DiskCleanupDialog, forward: bool) {
    let n = dialog.candidates.len();
    dialog.field = match (dialog.field, forward) {
        (CleanupField::Item(i), true) if i + 1 < n => CleanupField::Item(i + 1),
        (CleanupField::Item(_), true) | (CleanupField::Cancel, false) => CleanupField::Run,
        (CleanupField::Run, true) | (CleanupField::Item(0), false) => CleanupField::Cancel,
        (CleanupField::Cancel, true) => CleanupField::Item(0),
        (CleanupField::Run, false) => CleanupField::Item(n.saturating_sub(1)),
        (CleanupField::Item(i), false) => CleanupField::Item(i - 1),
    };
}

/// Toggle the checkbox for the currently focused item (if it exists on disk).
pub fn toggle_checked(dialog: &mut DiskCleanupDialog) {
    if let CleanupField::Item(idx) = dialog.field {
        if let Some(c) = dialog.candidates.get_mut(idx) {
            if c.exists {
                c.checked = !c.checked;
            }
        }
    }
}

// ── convenience wrappers used by popup handler ────────────────────────────────

/// Toggle the focused item then advance to the next field.
pub fn toggle_and_advance(app: &mut App) {
    let Overlay::DiskCleanup(ref mut dialog) = app.overlay else {
        return;
    };
    toggle_checked(dialog);
    advance_field(dialog, true);
}

/// Advance without toggling (arrow keys / Tab on a button).
pub fn move_focus(app: &mut App, forward: bool) {
    let Overlay::DiskCleanup(ref mut dialog) = app.overlay else {
        return;
    };
    advance_field(dialog, forward);
}

/// Space on a checkbox row.
pub fn space_checked(app: &mut App) {
    let Overlay::DiskCleanup(ref mut dialog) = app.overlay else {
        return;
    };
    toggle_checked(dialog);
}

/// Returns the current focused field without borrowing `app` for the whole
/// match expression in the popup handler.
pub fn current_field(app: &App) -> Option<CleanupField> {
    let Overlay::DiskCleanup(ref dialog) = app.overlay else {
        return None;
    };
    Some(dialog.field)
}
