//! Purpose: A tree panel with no room for rows draws its frame alone, a row
//! under the last child of its parent carries a gap, not a line, in its guide,
//! and a panel that is not a tree draws nothing at all.

use std::path::{Path, PathBuf};

use ratatui::Terminal;

use super::render;
use crate::models::{FolderTree, Panel, PanelKind};
use crate::test_support::{buffer_text, WritableTestBackend};

/// `/` holds `etc` and `var`; `var` holds `log`; nothing else exists.
fn lister(dir: &Path) -> Vec<PathBuf> {
    let children: &[&str] = match dir.to_str().unwrap_or_default() {
        "/" => &["/etc", "/var"],
        "/var" => &["/var/log"],
        _ => &[],
    };
    children.iter().map(PathBuf::from).collect()
}

fn panel_showing(path: &str) -> Panel {
    let mut tree = FolderTree::rooted_at(PathBuf::from("/"));
    tree.reveal(Path::new(path), &mut lister);
    let mut panel = Panel::from_entries(std::env::temp_dir(), Vec::new());
    panel.kind = PanelKind::Tree(Box::new(tree));
    panel
}

#[test]
fn a_tree_with_no_room_for_rows_draws_its_frame_alone() {
    let mut panel = panel_showing("/etc");
    let mut terminal = Terminal::new(WritableTestBackend::new(30, 3)).expect("terminal");
    terminal
        .draw(|f| render(f, &mut panel, f.area(), false, None))
        .expect("draw");
    let screen = buffer_text(&terminal);
    assert!(!screen.contains("etc"), "{screen}");
}

#[test]
fn a_row_under_the_last_child_is_indented_by_a_gap() {
    let mut panel = panel_showing("/var/log");
    let mut terminal = Terminal::new(WritableTestBackend::new(40, 12)).expect("terminal");
    terminal
        .draw(|f| render(f, &mut panel, f.area(), false, None))
        .expect("draw");
    let screen = buffer_text(&terminal);
    assert!(screen.contains("  └─log"), "{screen}");
}

#[test]
fn a_panel_that_is_not_a_tree_draws_nothing() {
    let mut panel = Panel::from_entries(std::env::temp_dir(), Vec::new());
    panel.kind = PanelKind::QuickView(Box::default());
    let mut terminal = Terminal::new(WritableTestBackend::new(30, 6)).expect("terminal");
    terminal
        .draw(|f| render(f, &mut panel, f.area(), false, None))
        .expect("draw");
    assert_eq!(buffer_text(&terminal).trim(), "");
}
