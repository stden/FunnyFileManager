//! Purpose: an action that arrives without a key — a menu row — over a tree, a
//! quick view or a host: the panel's kind reads it first, as it reads a key.

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::action::ToolCommand;
use crate::menu::MenuCommand;
use crate::models::{Panel, Viewport};
use crate::test_support::netbox::app_on_host;

fn terminal() -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal")
}

/// The left panel as a tree (`Ctrl+T`) over a folder with two subfolders.
fn tree_app() -> (TempDir, App) {
    let dir = TempDir::new().expect("temp dir");
    fs::create_dir(dir.path().join("alpha")).expect("alpha");
    fs::create_dir(dir.path().join("beta")).expect("beta");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_viewport(Viewport::new(24));
    ops::tree::toggle(&mut app);
    assert!(app.get_active_panel_ref().kind.is_tree());
    (dir, app)
}

fn tree_selection(app: &mut App) -> Option<PathBuf> {
    ops::tree::tree_mut(app)
        .and_then(|tree| tree.selected())
        .map(Path::to_path_buf)
}

#[test]
fn over_a_tree_the_cursor_moves_are_the_trees() {
    let (_dir, mut app) = tree_app();
    let mut terminal = terminal();
    dispatch_action(&mut terminal, &mut app, Action::First);
    let top = tree_selection(&mut app);
    dispatch_action(&mut terminal, &mut app, Action::NavDown);
    assert_ne!(tree_selection(&mut app), top, "the tree's cursor moved");
    assert_eq!(
        app.left_panel.selected(),
        Some(0),
        "the hidden list did not"
    );
}

/// Files → Copy over a tree: the row is told to wait for a file panel, as F5
/// is, and no copy prompt opens on the hidden entry.
#[test]
fn over_a_tree_a_file_operation_needs_a_file_panel() {
    let (_dir, mut app) = tree_app();
    dispatch_action(&mut terminal(), &mut app, Action::Copy);
    assert!(!app.overlay.is_active());
    assert_eq!(
        app.status_text(),
        "Tree panel: press Enter to open the folder, then F5–F8 work there"
    );
}

#[test]
fn over_a_tree_the_programs_rows_still_run() {
    let (_dir, mut app) = tree_app();
    dispatch_action(
        &mut terminal(),
        &mut app,
        Action::Menu(MenuCommand::Shortcuts),
    );
    assert!(matches!(app.overlay, Overlay::MenuDialog(_)));
    assert!(
        app.get_active_panel_ref().kind.is_tree(),
        "and the tree stays"
    );
}

#[test]
fn over_a_quick_view_a_cursor_move_scrolls_the_preview() {
    let dir = TempDir::new().expect("temp dir");
    let body: Vec<String> = (1..=40).map(|n| format!("line {n}")).collect();
    fs::write(dir.path().join("long.txt"), body.join("\n")).expect("write");
    let panel = Panel::new(dir.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_viewport(Viewport::new(24));
    assert!(app.get_active_panel().select_named("long.txt"));
    ops::quickview::toggle(&mut app);
    app.switch_panel();
    assert!(app.get_active_panel_ref().kind.is_quick_view());

    dispatch_action(&mut terminal(), &mut app, Action::NavDown);
    let PanelKind::QuickView(view) = &app.get_active_panel_ref().kind else {
        panic!("still a quick view");
    };
    assert_eq!(view.scroll.lines(), 1);
}

/// Over a host the Files rows reach for the local files hidden beneath the
/// panel and are refused; a program's row still runs.
#[test]
fn over_a_host_a_files_row_is_refused_and_a_programs_row_runs() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_on_host(dir.path(), &[("/etc/hostname", 5)], None);
    let mut terminal = terminal();
    dispatch_action(&mut terminal, &mut app, Action::View);
    assert!(!app.overlay.is_active());
    assert_eq!(
        app.status_text(),
        "NetBox: this command works on files; press Enter on `..` to return to them"
    );
    dispatch_action(&mut terminal, &mut app, Action::Tool(ToolCommand::History));
    assert!(matches!(app.overlay, Overlay::MenuDialog(_)));
}
