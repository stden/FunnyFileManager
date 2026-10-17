//! Purpose: Ctrl+F1/F2, Tab, the F9 bar and Ctrl+U never reach a hidden panel.

use super::*;

/// The keys themselves, through the dispatcher: `Ctrl+F2` hides the right
/// panel, and `Tab` / `Right` then leave the cursor on the left one.
#[test]
fn ctrl_f2_then_tab_and_right_keep_the_cursor_on_the_left_panel() {
    let mut app = app();
    let mut terminal = terminal();
    app.active_panel = PanelSide::Left;
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::F(2), KeyModifiers::CONTROL),
    );
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Tab));
    assert_eq!(app.active_panel, PanelSide::Left, "Tab is ignored");
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Right));
    assert_eq!(app.active_panel, PanelSide::Left, "Right is ignored");
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::F(2), KeyModifiers::CONTROL),
    );
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Tab));
    assert_eq!(
        app.active_panel,
        PanelSide::Right,
        "Tab works once both are shown"
    );
}

/// `Ctrl+F1` on the active left panel: the right one takes the cursor.
#[test]
fn ctrl_f1_on_the_active_panel_hands_the_cursor_over() {
    let mut app = app();
    let mut terminal = terminal();
    app.active_panel = PanelSide::Left;
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::F(1), KeyModifiers::CONTROL),
    );
    assert_eq!(app.panels_shown, PanelsShown::RightOnly);
    assert_eq!(app.active_panel, PanelSide::Right);
}

/// The F9 menu row is the same toggle as the key: choosing "Panel On/Off"
/// under the Right heading hides the right panel, and the row's tick follows.
#[test]
fn the_menu_row_is_the_same_toggle_as_the_key() {
    let mut app = app();
    crate::ops::menu::execute(
        &mut app,
        crate::menu::MenuCommand::TogglePanel(PanelSide::Right),
    );
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    assert_eq!(app.status.to_string(), "Right panel off");
    assert!(!panel_on_ticked(&app, Heading::Right));
}

/// `F9` then `Left` / `Right` walks the bar over the Left and Right headings.
/// Walking never moves the cursor to another panel; when the panel is hidden
/// its drop-down still opens (its "Panel On/Off" row is how the panel comes
/// back) and the cursor stays on the panel that is on screen.
#[test]
fn the_f9_bar_never_walks_the_cursor_onto_a_hidden_panel() {
    let mut app = app();
    let mut terminal = terminal();
    app.active_panel = PanelSide::Left;
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::F(2), KeyModifiers::CONTROL),
    );
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::F(9)));
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Left));
    assert!(on_right_heading(&app), "the bar wraps onto Right");
    assert_eq!(app.active_panel, PanelSide::Left, "wrapping onto Right");
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Esc));

    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::F(9)));
    for _ in 0..4 {
        dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Right));
    }
    assert!(
        on_right_heading(&app),
        "Left, Files, Commands, Options, Right"
    );
    assert_eq!(app.active_panel, PanelSide::Left, "walking onto Right");
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Esc));
    assert_eq!(app.active_panel, PanelSide::Left);
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
}

fn on_right_heading(app: &App) -> bool {
    matches!(app.overlay, Overlay::Menu(menu) if menu.heading == Heading::Right)
}

/// Far's `SwapPanels` does nothing unless both panels are visible: swapping
/// would put the hidden panel's directory on screen under the other's name.
#[test]
fn ctrl_u_is_ignored_while_a_panel_is_hidden() {
    let root = tempfile::tempdir().expect("temp dir");
    let (left_dir, right_dir) = two_directories(root.path());
    let mut app = app();
    let mut terminal = terminal();
    app.left_panel
        .navigate_to(left_dir.clone())
        .expect("left panel");
    app.right_panel
        .navigate_to(right_dir.clone())
        .expect("right panel");
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::F(2), KeyModifiers::CONTROL),
    );
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    assert_eq!(app.left_panel.path, left_dir, "the swap is refused");
    assert_eq!(app.right_panel.path, right_dir);
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    assert_eq!(
        app.status.to_string(),
        "Panels are not swapped while one is hidden (Ctrl+F2 shows the right panel)"
    );
    assert!(!draw(&mut app).contains("right_side_dir"));
}

/// With both panels on screen `Ctrl+U` swaps as it always has.
#[test]
fn ctrl_u_swaps_when_both_panels_are_shown() {
    let root = tempfile::tempdir().expect("temp dir");
    let (left_dir, right_dir) = two_directories(root.path());
    let mut app = app();
    app.left_panel
        .navigate_to(left_dir.clone())
        .expect("left panel");
    app.right_panel
        .navigate_to(right_dir.clone())
        .expect("right panel");
    swap_panels(&mut app);
    assert_eq!(app.left_panel.path, right_dir);
    assert_eq!(app.right_panel.path, left_dir);
    assert_eq!(app.status.to_string(), "Panels swapped");
}
