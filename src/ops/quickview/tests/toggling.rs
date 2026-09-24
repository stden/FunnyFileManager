//! Purpose: Ctrl+Q on and off, and a preview that follows the cursor or the tree.

use super::*;

#[test]
fn ctrl_q_previews_the_active_cursor_in_the_passive_panel() {
    let dir = fixture();
    let mut app = app_in(&dir);
    select(&mut app, "alpha.txt");

    toggle(&mut app);

    assert_eq!(
        app.active_panel,
        PanelSide::Left,
        "focus stays on the files"
    );
    assert_eq!(app.left_panel.kind, PanelKind::Files);
    let Preview::Text(text) = preview(&app, PanelSide::Right) else {
        panic!("expected text: {:?}", preview(&app, PanelSide::Right));
    };
    assert_eq!(text.lines, vec!["alpha one", "alpha two"]);
    assert_eq!(app.status.to_string(), "Quick view on");
}

#[test]
fn ctrl_q_again_brings_the_file_list_back() {
    let dir = fixture();
    let mut app = app_in(&dir);
    toggle(&mut app);
    toggle(&mut app);
    assert_eq!(app.right_panel.kind, PanelKind::Files);
    assert_eq!(app.status.to_string(), "Quick view off");
}

#[test]
fn ctrl_q_inside_the_preview_turns_that_panel_back_into_files() {
    let dir = fixture();
    let mut app = app_in(&dir);
    toggle(&mut app);
    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);

    toggle(&mut app);

    assert_eq!(app.right_panel.kind, PanelKind::Files);
    assert_eq!(app.left_panel.kind, PanelKind::Files);
}

#[test]
fn the_preview_follows_the_cursor_after_a_refresh() {
    let dir = fixture();
    let mut app = app_in(&dir);
    select(&mut app, "alpha.txt");
    toggle(&mut app);

    select(&mut app, "beta.rs");
    refresh(&mut app);

    let Preview::Text(text) = preview(&app, PanelSide::Right) else {
        panic!("expected text");
    };
    assert_eq!(text.lines, vec!["fn beta() {}"]);
    let PanelKind::QuickView(view) = &app.right_panel.kind else {
        panic!("not a quick view");
    };
    assert_eq!(
        view.shown.as_deref(),
        Some(dir.path().join("beta.rs").as_path())
    );
}

#[test]
fn a_refresh_on_the_same_file_keeps_the_scroll() {
    let dir = fixture();
    let mut app = app_in(&dir);
    select(&mut app, "alpha.txt");
    toggle(&mut app);
    if let PanelKind::QuickView(view) = &mut app.right_panel.kind {
        view.scroll_down(1, 1);
    }

    refresh(&mut app);

    let PanelKind::QuickView(view) = &app.right_panel.kind else {
        panic!("not a quick view");
    };
    assert_eq!(view.scroll.lines(), 1);
}

#[test]
fn a_preview_opposite_a_tree_follows_the_tree_cursor() {
    let dir = fixture();
    let mut app = app_in(&dir);
    toggle(&mut app);
    app.left_panel.kind = PanelKind::Tree(Box::new(crate::models::FolderTree::rooted_at(
        dir.path().join("nested"),
    )));

    refresh(&mut app);

    let Preview::Folder(stats) = preview(&app, PanelSide::Right) else {
        panic!("expected folder totals");
    };
    assert_eq!(stats.files, 2);
}
