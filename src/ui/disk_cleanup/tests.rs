//! Purpose: Cleanup checklist text, disabled rows, and keyboard focus rendering.

use ratatui::{buffer::Buffer, Terminal};
use tempfile::TempDir;

use crate::models::{App, CleanupField, CleanupKind, DiskCleanupDialog, FileSize, Overlay};
use crate::test_support::disk_cleanup::{app, candidate};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::{palette, Cells};

fn draw(app: &mut App) -> Terminal<WritableTestBackend> {
    let mut terminal = Terminal::new(WritableTestBackend::new(100, 24)).unwrap();
    terminal.draw(|frame| crate::ui::ui(frame, app)).unwrap();
    terminal
}

fn locate(buffer: &Buffer, label: &str) -> (u16, u16) {
    for y in 0..buffer.area.height {
        let text: String = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect();
        if let Some((before, _)) = text.split_once(label) {
            return (u16::try_from(Cells::of(before).get()).unwrap(), y);
        }
    }
    panic!("missing {label}");
}

#[test]
fn checklist_shows_sizes_pending_measurement_and_dimmed_missing_rows() {
    let temp = TempDir::new().unwrap();
    let mut selected = candidate("Selected", temp.path());
    selected.size = Some(FileSize::from_bytes(7));
    let mut missing = candidate("Missing", temp.path());
    missing.exists = false;
    missing.checked = false;
    let pending = candidate("Pending", temp.path());
    let mut app = app(temp.path(), vec![selected, missing, pending]);
    let terminal = draw(&mut app);
    let text = buffer_text(&terminal);
    let path = temp.path().display();
    for label in [
        "Disk Cleanup".to_owned(),
        format!("[x] Selected ({path})  [7 B]"),
        format!("[ ] Missing ({path})  [not found]"),
        format!("Pending ({path})  [measuring…]"),
        "Clean up".to_owned(),
        "Cancel".to_owned(),
    ] {
        assert!(text.contains(&label), "missing {label}: {text}");
    }
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[locate(buffer, "Selected")].bg,
        palette::DIALOG_FOCUS_BG
    );
    assert_eq!(buffer[locate(buffer, "Missing")].fg, palette::DISABLED_FG);
}

#[test]
fn button_focus_moves_without_highlighting_the_checklist() {
    let temp = TempDir::new().unwrap();
    let mut app = app(temp.path(), vec![candidate("Cache", temp.path())]);
    for (field, label) in [
        (CleanupField::Run, "Clean up"),
        (CleanupField::Cancel, "Cancel"),
    ] {
        let Overlay::DiskCleanup(dialog) = &mut app.overlay else {
            panic!("cleanup")
        };
        dialog.field = field;
        let terminal = draw(&mut app);
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[locate(buffer, label)].bg, palette::DIALOG_FOCUS_BG);
        assert_ne!(buffer[locate(buffer, "Cache")].bg, palette::DIALOG_FOCUS_BG);
    }
}

/// Each row names the folder it measures and deletes, so a label can never
/// promise `~/.cache/pip` while the dialog scans somewhere else.
#[test]
fn every_default_row_shows_the_path_it_scans() {
    let temp = TempDir::new().unwrap();
    let rows = DiskCleanupDialog::new().candidates;
    let mut app = app(temp.path(), rows.clone());
    let text = buffer_text(&draw(&mut app));
    for row in &rows {
        let scanned = if row.kind == CleanupKind::HeapDumps {
            row.path.join("*.hprof")
        } else {
            row.path.clone()
        };
        let scanned = scanned.display().to_string();
        assert!(
            text.contains(&scanned),
            "{} does not show {scanned}: {text}",
            row.label
        );
    }
}
