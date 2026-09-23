//! Purpose: the Disk Cleanup dialog's state and cursor, exercised through the
//! public API.
//!
//! Everything here goes through `ops::disk_cleanup` and `models`, so the tests
//! describe what `:cleanup` does rather than how the dialog stores it. The
//! candidate list is replaced with fixtures under a `TempDir`: the real one
//! points at machine-specific caches, which would make the results depend on
//! the developer's home directory.

use std::fs;
use std::path::{Path, PathBuf};

use funny_file_manager::models::{
    App, CleanupCandidate, CleanupField, CleanupKind, DiskCleanupDialog, FileSize, Overlay, Panel,
};
use funny_file_manager::ops::disk_cleanup;
use tempfile::TempDir;

fn candidate(label: &'static str, path: impl Into<PathBuf>) -> CleanupCandidate {
    CleanupCandidate {
        label,
        path: path.into(),
        kind: CleanupKind::Path,
        size: None,
        checked: false,
        exists: false,
    }
}

/// A dialog carrying only the given candidates, with the cursor on the first.
fn dialog_with(candidates: Vec<CleanupCandidate>) -> DiskCleanupDialog {
    let mut dialog = DiskCleanupDialog::new();
    dialog.candidates = candidates;
    dialog.field = CleanupField::Item(0);
    dialog
}

fn app_with(temp: &TempDir, dialog: DiskCleanupDialog) -> App {
    let panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::DiskCleanup(Box::new(dialog)));
    app
}

fn dialog_of(app: &App) -> &DiskCleanupDialog {
    let Overlay::DiskCleanup(ref dialog) = app.overlay else {
        panic!("the cleanup dialog must still be open");
    };
    dialog
}

/// Four rows covering every combination the two selection queries care about.
fn mixed_candidates() -> Vec<CleanupCandidate> {
    vec![
        CleanupCandidate {
            size: Some(FileSize::from_bytes(1_000)),
            checked: true,
            exists: true,
            ..candidate("ticked", "/cache/ticked")
        },
        CleanupCandidate {
            size: Some(FileSize::from_bytes(2_000)),
            exists: true,
            ..candidate("unticked", "/cache/unticked")
        },
        // Ticked but absent from this machine: the dialog shows it dimmed, and
        // it must not be counted or handed to the delete step.
        CleanupCandidate {
            size: Some(FileSize::from_bytes(4_000)),
            checked: true,
            ..candidate("missing", "/cache/missing")
        },
        // Ticked and present, but never measured.
        CleanupCandidate {
            checked: true,
            exists: true,
            ..candidate("unmeasured", "/cache/unmeasured")
        },
    ]
}

#[test]
fn selected_paths_skip_unchecked_and_missing_candidates() {
    let dialog = dialog_with(mixed_candidates());

    assert_eq!(
        dialog.selected_paths(),
        vec![
            PathBuf::from("/cache/ticked"),
            PathBuf::from("/cache/unmeasured"),
        ],
    );
}

#[test]
fn selected_bytes_count_only_what_was_measured() {
    let dialog = dialog_with(mixed_candidates());

    assert_eq!(
        dialog.selected_bytes(),
        1_000,
        "an unmeasured candidate contributes no bytes, but is still removed"
    );
}

#[test]
fn measure_resolves_relative_paths_against_the_base() {
    let temp = TempDir::new().expect("test setup");
    let target = temp.path().join("target");
    fs::create_dir(&target).expect("test setup");
    fs::write(target.join("artifact.bin"), vec![0_u8; 512]).expect("test setup");

    let mut dialog = dialog_with(vec![candidate("Rust target", "target")]);
    disk_cleanup::measure(&mut dialog, temp.path());

    let measured = &dialog.candidates[0];
    assert_eq!(
        measured.path, target,
        "the relative path is stored back as an absolute one"
    );
    assert!(measured.exists);
    assert_eq!(measured.size.map(FileSize::bytes), Some(512));
    assert!(
        !measured.checked,
        "only the Temp folder is pre-ticked; everything else is opt-in"
    );
}

#[test]
fn measure_leaves_a_candidate_that_is_not_on_this_machine_unsized() {
    let temp = TempDir::new().expect("test setup");
    let absent = temp.path().join("nothing-here");

    let mut dialog = dialog_with(vec![candidate("Docker cache", absent.clone())]);
    disk_cleanup::measure(&mut dialog, temp.path());

    let measured = &dialog.candidates[0];
    assert!(!measured.exists);
    assert_eq!(measured.size, None);
    assert!(!measured.checked);
    assert_eq!(measured.path, absent, "an absolute path is left alone");
}

#[test]
fn measure_pre_ticks_the_temp_folder() {
    let temp = TempDir::new().expect("test setup");
    let cache = temp.path().join("tmp-cache");
    fs::create_dir(&cache).expect("test setup");
    fs::write(cache.join("stale.log"), b"stale").expect("test setup");

    let row = CleanupCandidate {
        kind: CleanupKind::Contents,
        ..candidate("Temp files", cache)
    };
    let mut dialog = dialog_with(vec![row]);
    disk_cleanup::measure(&mut dialog, temp.path());

    assert!(
        dialog.candidates[0].checked,
        "emptying the Temp folder keeps the folder and other users' files, \
         so it starts ticked"
    );
    assert_eq!(dialog.selected_bytes(), 5);
}

#[test]
fn measure_sizes_only_the_heap_dumps_inside_the_home_directory() {
    let temp = TempDir::new().expect("test setup");
    let home = temp.path().join("home");
    fs::create_dir(&home).expect("test setup");
    fs::write(home.join("java_pid1.hprof"), vec![0_u8; 100]).expect("test setup");
    fs::write(home.join("java_pid2.hprof"), vec![0_u8; 200]).expect("test setup");
    // Sizing the whole home directory instead would sweep this in too.
    fs::write(home.join("notes.txt"), vec![0_u8; 9_000]).expect("test setup");

    let mut row = candidate("JVM heap dumps", home);
    row.kind = CleanupKind::HeapDumps;
    let mut dialog = dialog_with(vec![row]);
    disk_cleanup::measure(&mut dialog, temp.path());

    let measured = &dialog.candidates[0];
    assert!(measured.exists);
    assert_eq!(measured.size.map(FileSize::bytes), Some(300));
    assert!(
        measured.checked,
        "a heap dump is never wanted, so finding one pre-ticks the row"
    );
}

#[test]
fn advance_field_walks_the_rows_then_the_buttons_and_wraps() {
    let mut dialog = dialog_with(vec![candidate("first", "/a"), candidate("second", "/b")]);

    // Forward: every row in turn, then Clean up, then Cancel, then back round.
    let forward = [
        CleanupField::Item(1),
        CleanupField::Run,
        CleanupField::Cancel,
        CleanupField::Item(0),
    ];
    for expected in forward {
        disk_cleanup::advance_field(&mut dialog, true);
        assert_eq!(dialog.field, expected);
    }

    // Backward retraces the same ring in reverse.
    let backward = [
        CleanupField::Cancel,
        CleanupField::Run,
        CleanupField::Item(1),
        CleanupField::Item(0),
    ];
    for expected in backward {
        disk_cleanup::advance_field(&mut dialog, false);
        assert_eq!(dialog.field, expected);
    }
}

#[test]
fn toggle_checked_refuses_a_row_that_is_not_on_this_machine() {
    let present = CleanupCandidate {
        exists: true,
        ..candidate("present", "/a")
    };
    let mut dialog = dialog_with(vec![present, candidate("absent", "/b")]);

    disk_cleanup::toggle_checked(&mut dialog);
    assert!(dialog.candidates[0].checked, "a present row ticks");
    disk_cleanup::toggle_checked(&mut dialog);
    assert!(!dialog.candidates[0].checked, "and unticks again");

    dialog.field = CleanupField::Item(1);
    disk_cleanup::toggle_checked(&mut dialog);
    assert!(
        !dialog.candidates[1].checked,
        "a dimmed row cannot be selected for deletion"
    );

    // On a button there is no checkbox to flip, and nothing may change.
    dialog.field = CleanupField::Run;
    disk_cleanup::toggle_checked(&mut dialog);
    assert!(dialog.candidates.iter().all(|c| !c.checked));
}

#[test]
fn toggle_and_advance_ticks_the_row_under_the_cursor_then_moves_on() {
    let temp = TempDir::new().expect("test setup");
    let rows = vec![
        CleanupCandidate {
            exists: true,
            ..candidate("first", "/a")
        },
        CleanupCandidate {
            exists: true,
            ..candidate("second", "/b")
        },
    ];
    let mut app = app_with(&temp, dialog_with(rows));

    disk_cleanup::toggle_and_advance(&mut app);

    let dialog = dialog_of(&app);
    assert!(dialog.candidates[0].checked);
    assert_eq!(dialog.field, CleanupField::Item(1));
}

#[test]
fn space_checked_ticks_the_row_without_moving_the_cursor() {
    let temp = TempDir::new().expect("test setup");
    let rows = vec![CleanupCandidate {
        exists: true,
        ..candidate("only", "/a")
    }];
    let mut app = app_with(&temp, dialog_with(rows));

    disk_cleanup::space_checked(&mut app);

    let dialog = dialog_of(&app);
    assert!(dialog.candidates[0].checked);
    assert_eq!(
        dialog.field,
        CleanupField::Item(0),
        "Space ticks in place; only Enter and the arrows move"
    );
}

#[test]
fn move_focus_walks_the_dialog_in_both_directions() {
    let temp = TempDir::new().expect("test setup");
    let rows = vec![candidate("first", "/a"), candidate("second", "/b")];
    let mut app = app_with(&temp, dialog_with(rows));

    disk_cleanup::move_focus(&mut app, true);
    assert_eq!(dialog_of(&app).field, CleanupField::Item(1));

    disk_cleanup::move_focus(&mut app, false);
    assert_eq!(dialog_of(&app).field, CleanupField::Item(0));

    assert!(
        dialog_of(&app).candidates.iter().all(|c| !c.checked),
        "moving the cursor never ticks anything"
    );
}

#[test]
fn the_dialog_wrappers_ignore_every_other_overlay() {
    let temp = TempDir::new().expect("test setup");
    let panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_overlay(Overlay::Help);

    // Each wrapper reads the overlay itself, so a stale key for a dialog that
    // has already closed must fall through rather than panic.
    disk_cleanup::toggle_and_advance(&mut app);
    disk_cleanup::space_checked(&mut app);
    disk_cleanup::move_focus(&mut app, true);

    assert!(matches!(app.overlay, Overlay::Help));
}

#[test]
fn a_fresh_dialog_offers_candidates_with_nothing_selected() {
    let dialog = DiskCleanupDialog::new();

    assert!(
        !dialog.candidates.is_empty(),
        "the checklist is built from the well-known cache locations"
    );
    assert_eq!(dialog.field, CleanupField::Item(0));
    assert!(
        dialog.candidates.iter().all(|c| c.size.is_none()),
        "sizes stay empty until `measure` runs"
    );
    assert_eq!(dialog.selected_paths(), Vec::<PathBuf>::new());
    assert_eq!(dialog.selected_bytes(), 0);
}

#[test]
fn measure_is_reported_against_the_active_panel_directory() {
    // `open` resolves relative candidates against the active panel, so the same
    // relative name measures differently from two different directories.
    let first = TempDir::new().expect("test setup");
    let second = TempDir::new().expect("test setup");
    for (dir, bytes) in [(&first, 64_usize), (&second, 256_usize)] {
        let target: &Path = &dir.path().join("target");
        fs::create_dir(target).expect("test setup");
        fs::write(target.join("blob"), vec![0_u8; bytes]).expect("test setup");
    }

    let mut from_first = dialog_with(vec![candidate("Rust target", "target")]);
    disk_cleanup::measure(&mut from_first, first.path());
    let mut from_second = dialog_with(vec![candidate("Rust target", "target")]);
    disk_cleanup::measure(&mut from_second, second.path());

    assert_eq!(from_first.candidates[0].size.map(FileSize::bytes), Some(64));
    assert_eq!(
        from_second.candidates[0].size.map(FileSize::bytes),
        Some(256)
    );
}

/// On Linux, pip and Playwright keep their caches in the XDG cache folder
/// (`~/.cache`), which is where the rows must look — not in a Windows-style
/// `AppData` folder that no Linux tool creates.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn linux_cache_rows_scan_the_xdg_cache_folder() {
    let dialog = DiskCleanupDialog::new();
    for folder in ["pip", "ms-playwright", "JetBrains"] {
        let row = dialog
            .candidates
            .iter()
            .find(|row| row.path.ends_with(folder))
            .unwrap_or_else(|| panic!("no row scans a {folder} folder"));
        assert!(
            !row.path
                .components()
                .any(|part| part.as_os_str() == "AppData"),
            "{} scans {}",
            row.label,
            row.path.display()
        );
    }
}
