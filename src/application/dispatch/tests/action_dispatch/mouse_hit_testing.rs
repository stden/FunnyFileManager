//! Purpose: mouse hit-testing over panel columns, hidden panels and the sort menu.

use super::*;

#[test]
fn sort_menu_mouse_click_activates_the_clicked_criterion() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["file"]);
    ops::sortmenu::open(&mut app);
    let mut clicks = crate::mouse::PanelClicks::default();
    let screen = ratatui::layout::Rect::new(0, 0, 80, 24);
    let mut mouse = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::NONE,
    };
    dispatch_mouse(&mut terminal, &mut app, mouse, screen, &mut clicks);
    assert!(app.overlay.is_active());
    // The box is centred and sized to its rows, so the row's place is read
    // back from the frame rather than written here.
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let area = app.dialog_mouse.area.expect("the menu was drawn");
    mouse.column = area.x + 2;
    // Size is the fourth row Far lists, after name, extension and time.
    mouse.row = area.y + 1 + 3;
    dispatch_mouse(&mut terminal, &mut app, mouse, screen, &mut clicks);
    assert!(!app.overlay.is_active());
    assert_eq!(app.left_panel.sort_mode, models::SortMode::Size);
}

#[test]
fn hidden_panels_and_the_frame_do_not_select_files() {
    let (mut app, _terminal, _dir) = app_and_terminal(&["file"]);
    let screen = ratatui::layout::Rect::new(0, 0, 80, 24);
    let row = crate::mouse::Target::PanelRow {
        side: PanelSide::Left,
        row: 0,
    };
    app.left_panel.view = models::PanelView::Brief;
    // Column 39 is the left panel's right frame; the columns reach it, with
    // no padding left over for a click to fall into.
    assert_eq!(
        panel_mouse::target(&app, row, screen, 39),
        crate::mouse::Target::Nothing
    );
    assert_eq!(panel_mouse::page(&app, 16), 48);
    app.left_panel.view = models::PanelView::Medium;
    assert_eq!(panel_mouse::page(&app, 16), 32);
}

/// A panel alone on screen (`Ctrl+F1` hid the other) is drawn over the whole
/// width, so its Brief columns are a third of the screen, not of a half: the
/// column that was padding at half width is the second column now.
#[test]
fn a_panel_alone_on_screen_spreads_its_columns_over_the_full_width() {
    let (mut app, _terminal, _dir) = app_and_terminal(&["file"]);
    let screen = ratatui::layout::Rect::new(0, 0, 80, 24);
    let row = crate::mouse::Target::PanelRow {
        side: PanelSide::Right,
        row: 0,
    };
    app.right_panel.view = models::PanelView::Brief;
    app.active_panel = PanelSide::Right;
    app.panels_shown = models::PanelsShown::RightOnly;
    assert_eq!(
        panel_mouse::target(&app, row, screen, 38),
        crate::mouse::Target::PanelRow {
            side: PanelSide::Right,
            row: 16,
        }
    );
    app.panels_shown = models::PanelsShown::Both;
    // Column 38 sits in the left half; the right panel starts at the split.
    assert_eq!(
        panel_mouse::target(&app, row, screen, 38),
        crate::mouse::Target::Nothing
    );
}

#[test]
fn brief_panel_click_selects_the_file_in_the_clicked_column() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&[]);
    for index in 0..45 {
        fs::write(app.left_panel.path.join(format!("file{index:02}")), "x").unwrap();
    }
    app.left_panel.load_entries().unwrap();
    app.left_panel.view = models::PanelView::Brief;
    terminal.draw(|frame| ui(frame, &mut app)).unwrap();
    let mouse = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 14,
        row: 3,
        modifiers: KeyModifiers::NONE,
    };
    dispatch_mouse(
        &mut terminal,
        &mut app,
        mouse,
        ratatui::layout::Rect::new(0, 0, 80, 24),
        &mut crate::mouse::PanelClicks::default(),
    );
    assert_eq!(app.left_panel.selected(), Some(16));
}

#[test]
fn non_file_panels_ignore_brief_stripes_and_keep_quickview_safe() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt", "b.txt"]);
    let screen = ratatui::layout::Rect::new(0, 0, 80, 24);
    app.left_panel.view = models::PanelView::Brief;

    app.left_panel.kind = PanelKind::Sessions(Box::new(models::SessionsPanel::from_store(
        &crate::sessions::SessionStore::default(),
    )));
    assert_eq!(panel_mouse::page(&app, 16), 16);
    navigation::select_panel_or_column(&terminal, &mut app, PanelSide::Right);
    assert_eq!(app.active_panel, PanelSide::Right);

    app.left_panel.kind = PanelKind::QuickView(Box::default());
    app.left_panel.select(Some(0));
    let mut clicks = crate::mouse::PanelClicks::default();
    let click = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 25,
        row: 4,
        modifiers: KeyModifiers::NONE,
    };
    dispatch_mouse(&mut terminal, &mut app, click, screen, &mut clicks);
    assert_eq!(
        (app.active_panel, app.left_panel.selected()),
        (PanelSide::Left, Some(0))
    );
}

#[test]
fn tree_panel_routes_clicks_to_headerless_tree_rows() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt", "b.txt"]);
    let screen = ratatui::layout::Rect::new(0, 0, 80, 24);
    let sub = app.left_panel.path.join("sub");
    fs::create_dir_all(&sub).unwrap();
    let mut tree = models::FolderTree::rooted_at(app.left_panel.path.clone());
    tree.expand(&mut |_| vec![sub.clone()]);
    app.left_panel.kind = PanelKind::Tree(Box::new(tree));
    let row0 = crate::mouse::Target::PanelRow {
        side: PanelSide::Left,
        row: 0,
    };
    assert_eq!(panel_mouse::hit(&app, screen, 25, 2), row0);
    let click = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 25,
        row: 3,
        modifiers: KeyModifiers::NONE,
    };
    let mut clicks = crate::mouse::PanelClicks::default();
    dispatch_mouse(&mut terminal, &mut app, click, screen, &mut clicks);
    let PanelKind::Tree(tree) = &app.left_panel.kind else {
        panic!("expected tree");
    };
    assert_eq!(tree.cursor, 1);
}
