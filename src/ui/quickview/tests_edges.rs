//! Purpose: The quick view on a panel too short for its footer, which keeps the
//! body alone, and on a folder walked completely, which claims no lower bounds.

use std::path::PathBuf;

use ratatui::Terminal;

use super::render;
use crate::models::{FileSize, Panel, PanelKind, Preview, QuickView, TextPreview};
use crate::stats::{Coverage, DirectoryStats};
use crate::test_support::{buffer_text, WritableTestBackend};

fn panel_showing(view: QuickView) -> Panel {
    let mut panel = Panel::new(std::env::temp_dir()).expect("test panel");
    panel.kind = PanelKind::QuickView(Box::new(view));
    panel
}

#[test]
fn a_panel_too_short_for_the_footer_keeps_only_the_body() {
    let path = PathBuf::from("/srv/x/notes.txt");
    let mut view = QuickView::default();
    view.show(
        Some(path.clone()),
        Preview::Text(TextPreview::decoded(&path, b"first\nsecond\nthird")),
    );
    let panel = panel_showing(view);
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 4)).expect("terminal");
    terminal
        .draw(|f| render(f, &panel, f.area(), false))
        .expect("draw");
    let screen = buffer_text(&terminal);
    assert!(screen.contains("first"), "{screen}");
    assert!(screen.contains("second"), "{screen}");
    assert!(!screen.contains("third"), "two body rows only: {screen}");
}

#[test]
fn a_folder_walked_completely_carries_no_lower_bound_note() {
    let mut view = QuickView::default();
    view.show(
        Some(PathBuf::from("/srv/x/project")),
        Preview::Folder(Box::new(DirectoryStats {
            total: FileSize::from_bytes(1024),
            files: 3,
            directories: 1,
            coverage: Coverage::Complete,
            ..DirectoryStats::default()
        })),
    );
    let panel = panel_showing(view);
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 12)).expect("terminal");
    terminal
        .draw(|f| render(f, &panel, f.area(), false))
        .expect("draw");
    let screen = buffer_text(&terminal);
    assert!(screen.contains("Files"), "{screen}");
    assert!(!screen.contains("lower bounds"), "{screen}");
}
