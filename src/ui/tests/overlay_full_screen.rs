//! Unit tests: the full-screen editor and viewer erase the panels behind
//! them, and the help window keeps Far's fixed console colours.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::*;
use crate::models::DirEntry;
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;

// -----------------------------------------------------------------------
// Overlays must erase the cells they cover, not just restyle them
// -----------------------------------------------------------------------

/// App whose panels are full of loud `X` rows that would show through any
/// overlay that restyles its area without clearing the old glyphs.
fn app_with_loud_panels() -> App {
    let mut app = App::new().expect("test app");
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    for panel in [&mut app.left_panel, &mut app.right_panel] {
        panel.entries.clear();
        for i in 0..20 {
            panel.entries.push(DirEntry {
                name: format!("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX_{i}"),
                kind: EntryKind::File,
                size: FileSize::from_bytes(0),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            });
        }
        panel.select(Some(0));
    }
    app
}

#[test]
fn test_editor_overlay_erases_the_panels_behind_it() {
    let mut app = app_with_loud_panels();
    app.set_overlay(Overlay::Editor(crate::models::Editor {
        title: "notes.txt".to_owned(),
        lines: vec!["hi".to_owned()],
        ..Default::default()
    }));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // The row holding the editor's short line must not carry panel glyphs
    // to its right — a Block restyles cells but does not erase their symbols.
    let buf = terminal.backend().buffer();
    // The editor is full screen, as Far's is.
    let area = buf.area;
    let row_y = area.y + 1; // first body row, where "hi" is drawn
                            // Borderless: the body runs the full width of the screen.
    let row: String = (area.x..area.x + area.width)
        .map(|x| buf[(x, row_y)].symbol())
        .collect();
    assert!(row.contains("hi"), "editor line missing: {row:?}");
    assert!(
        !row.contains('X'),
        "panel glyphs bleed through the editor: {row:?}"
    );
}

#[test]
fn test_viewer_overlay_erases_the_panels_behind_it() {
    let mut app = app_with_loud_panels();
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "log",
        vec!["hi".to_owned()],
    )));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let buf = terminal.backend().buffer();
    // The viewer is full screen, as Far's is: the body starts on the row
    // under the status line and runs the full width.
    let area = buf.area;
    let row_y = area.y + 1;
    let row: String = (area.x..area.x + area.width)
        .map(|x| buf[(x, row_y)].symbol())
        .collect();
    assert!(row.contains("hi"), "viewer line missing: {row:?}");
    let bar: String = (area.x..area.x + area.width)
        .map(|x| buf[(x, area.y)].symbol())
        .collect();
    assert!(
        !bar.contains('X'),
        "panel glyphs bleed through the bar: {bar:?}"
    );
    assert!(
        !row.contains('X'),
        "panel glyphs bleed through the viewer: {row:?}"
    );
}

/// After closing the F4 editor and rendering two more panel frames, the panel
/// must not display any characters that were written by the editor into
/// ratatui's alternating buffer A.
///
/// Root cause: ratatui alternates between two render buffers.  Frame 1 writes
/// RESIDUE symbols into buffer A.  Frame 2 (first panel frame) uses buffer B
/// (clean).  Frame 3 (second panel frame) reuses buffer A; `render_backdrop`
/// calls `set_style()` which sets the background colour but does NOT clear the
/// symbol, so RESIDUE symbols survive unless `Clear` is rendered first.
#[test]
fn test_panel_has_no_editor_residue_after_two_panel_frames() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    let mut app = App::new().unwrap();
    app.left_panel = crate::models::Panel::new(left.path().to_path_buf()).unwrap();
    app.right_panel = crate::models::Panel::new(right.path().to_path_buf()).unwrap();
    app.set_overlay(Overlay::Editor(crate::models::Editor {
        title: "test.txt".to_owned(),
        lines: (0..20).map(|i| format!("RESIDUE_{i}")).collect(),
        ..Default::default()
    }));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // Frame 1: editor active — writes RESIDUE symbols into buffer A.
    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Close the editor; subsequent frames render the panel view.
    app.close_overlay();

    // Frame 2: first panel frame — uses buffer B (clean, no residue).
    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Frame 3: second panel frame — ratatui switches back to buffer A.
    // Without `Clear`, render_backdrop only calls set_style and the old RESIDUE
    // symbols remain in every cell that panel rendering does not explicitly
    // overwrite.
    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let buf = terminal.backend().buffer();
    let row: String = (0..buf.area.width).map(|x| buf[(x, 10)].symbol()).collect();
    assert!(
        !row.contains("RESIDUE"),
        "editor residue visible on panel after two panel frames: {row:?}"
    );
}

#[test]
fn help_uses_fixed_far_console_colors() {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::Help);
    terminal
        .draw(|frame| render_overlays(frame, &mut app))
        .unwrap();
    let cell = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .find(|cell| cell.symbol() == "H")
        .unwrap();
    assert_eq!(cell.fg, Color::Rgb(0, 0, 0));
    assert_eq!(cell.bg, Color::Rgb(0, 128, 128));
}
