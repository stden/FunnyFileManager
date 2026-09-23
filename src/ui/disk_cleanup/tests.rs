//! Purpose: Cleanup checklist text, disabled rows, and keyboard focus rendering.

use ratatui::{buffer::Buffer, Terminal};
use tempfile::TempDir;

use crate::models::{App, CleanupField, FileSize, Overlay};
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
        if let Some(byte) = text.find(label) {
            return (u16::try_from(Cells::of(&text[..byte]).get()).unwrap(), y);
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
    for label in [
        "Disk Cleanup",
        "[x] Selected  [7 B]",
        "[ ] Missing  [not found]",
        "Pending  [measuring…]",
        "Clean up",
        "Cancel",
    ] {
        assert!(text.contains(label), "missing {label}: {text}");
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
