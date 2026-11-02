//! Purpose: clicks on a folder tree: its frame, the rows past its last entry, and a double-click that leaves the tree for a folder.

use super::*;

/// Opens the active panel's folder as a tree with its `sub` folder expanded,
/// so the tree has two rows: the root, then `sub`. Returns `sub`.
fn open_tree(app: &mut App) -> PathBuf {
    let sub = app.left_panel.path.join("sub");
    fs::create_dir_all(&sub).expect("test setup");
    let mut tree = models::FolderTree::rooted_at(app.left_panel.path.clone());
    tree.expand(&mut |_| vec![sub.clone()]);
    app.left_panel.kind = PanelKind::Tree(Box::new(tree));
    sub
}

fn tree_cursor(app: &App) -> usize {
    let PanelKind::Tree(tree) = &app.left_panel.kind else {
        panic!("expected the left panel to stay a tree");
    };
    tree.cursor
}

#[test]
fn a_click_on_a_tree_frame_activates_the_panel_and_moves_no_cursor() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&[]);
    open_tree(&mut app);
    app.active_panel = PanelSide::Right;
    terminal.draw(|frame| ui(frame, &mut app)).expect("draw");
    let mut clicks = crate::mouse::PanelClicks::default();

    // Row 1 is the frame above the tree rows, which start at row 2.
    click_at(&mut terminal, &mut app, (25, 1), &mut clicks);

    assert_eq!(app.active_panel, PanelSide::Left);
    assert_eq!(tree_cursor(&app), 0);
    assert!(matches!(app.left_panel.kind, PanelKind::Tree(_)));
}

#[test]
fn a_click_past_the_last_tree_row_breaks_a_pending_double_click() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&[]);
    open_tree(&mut app);
    terminal.draw(|frame| ui(frame, &mut app)).expect("draw");
    let mut clicks = crate::mouse::PanelClicks::default();

    // Row 3 is the second tree row (`sub`); row 10 is far below the last one.
    click_at(&mut terminal, &mut app, (25, 3), &mut clicks);
    click_at(&mut terminal, &mut app, (25, 10), &mut clicks);
    click_at(&mut terminal, &mut app, (25, 3), &mut clicks);

    assert_eq!(tree_cursor(&app), 1);
    assert!(
        matches!(app.left_panel.kind, PanelKind::Tree(_)),
        "the click after the blank row selects; it does not complete a double-click"
    );
}

#[test]
fn a_double_click_on_a_tree_row_leaves_the_tree_for_that_folder() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&[]);
    let sub = open_tree(&mut app);
    terminal.draw(|frame| ui(frame, &mut app)).expect("draw");
    let mut clicks = crate::mouse::PanelClicks::default();

    for _ in 0..2 {
        click_at(&mut terminal, &mut app, (25, 3), &mut clicks);
    }

    assert!(matches!(app.left_panel.kind, PanelKind::Files));
    assert_eq!(app.left_panel.path, sub);
}
