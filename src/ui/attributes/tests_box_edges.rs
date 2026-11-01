//! Purpose: A focused box the dialog cannot show is not highlighted, rather than
//! painted over the frame or the buttons, and a box with no place on the dialog
//! paints nothing.

use std::path::PathBuf;
use std::time::SystemTime;

use ratatui::style::Modifier;
use ratatui::Terminal;

use super::highlight_box;
use crate::models::{App, AttributesDialog, AttributesField, EntrySnapshot, Flag, Overlay, Panel};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::overlays::render_overlays;

fn snapshot() -> EntrySnapshot {
    EntrySnapshot {
        path: PathBuf::from("/tmp/a"),
        bits: 0,
        is_dir: false,
        owner: Some("alice".to_owned()),
        group: Some("staff".to_owned()),
        modified: Some(SystemTime::UNIX_EPOCH),
        accessed: Some(SystemTime::UNIX_EPOCH),
    }
}

fn reversed_cells(terminal: &Terminal<WritableTestBackend>) -> usize {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .filter(|cell| cell.style().add_modifier.contains(Modifier::REVERSED))
        .count()
}

#[test]
fn a_focused_box_past_the_dialog_edge_is_not_highlighted() {
    let mut dialog = AttributesDialog::from_snapshots(&[snapshot()]);
    dialog.field = AttributesField::Flag(Flag::GROUPS[0].flags.len() - 1);
    let mut terminal = Terminal::new(WritableTestBackend::new(30, 24)).expect("test terminal");
    let root = tempfile::TempDir::new().expect("test folder");
    let mut app = App::with_panels(
        Panel::new(root.path().to_owned()).expect("test panel"),
        Panel::new(root.path().to_owned()).expect("test panel"),
    );
    app.set_overlay(Overlay::Attributes(Box::new(dialog)));
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    let screen = buffer_text(&terminal);
    assert_eq!(reversed_cells(&terminal), 0, "{screen}");
}

#[test]
fn a_box_the_groups_do_not_place_is_not_highlighted() {
    let dialog = AttributesDialog::from_snapshots(&[snapshot()]);
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 12)).expect("test terminal");
    terminal
        .draw(|f| highlight_box(f, &dialog, &[], 0, f.area()))
        .expect("draw");
    assert_eq!(reversed_cells(&terminal), 0);
}
