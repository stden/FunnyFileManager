//! Purpose: Mouse on the key bar, the wheel, and clicked destructive actions.

use super::*;

/// Clicking `5Copy` must do exactly what F5 does — the click is translated into
/// the key, not into a second implementation of copying.
#[test]
fn e2e_clicking_the_key_bar_runs_the_function_key() {
    let (left, right) = fixture(&[("clicked.txt", "by mouse")]);

    // The bar is the last row; at 100 columns each of the twelve cells is 8
    // wide with the first four one wider, so F5 covers columns 36..44.
    // Clicking 5Copy opens the Copy dialog, exactly as pressing F5 does;
    // Enter then accepts the offered destination.
    let mut events = vec![
        key(KeyCode::Down),
        click(38, SCREEN_H - 1),
        key(KeyCode::Enter),
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        right.path().join("clicked.txt").exists(),
        "clicking 5Copy must copy, as F5 does"
    );
}

/// Mouse modifiers are part of the gesture. Shift-clicking `4Edit` therefore
/// means Shift+F4 (new file), not bare F4 (edit the selected entry).
#[test]
fn e2e_shift_clicking_the_key_bar_preserves_modifiers() {
    let (left, right) = fixture(&[("selected.txt", "leave me alone")]);

    // F4 covers columns 27..36 at this width.
    let screen = run_until(
        left.path(),
        right.path(),
        vec![click_with(30, SCREEN_H - 1, KeyModifiers::SHIFT)],
    );

    assert!(
        screen.contains("Create New File (Shift+F4)"),
        "Shift-click must take the Shift+F4 path:\n{screen}"
    );
}

/// Every painted key cell is a real control, not decoration. F2 opens the
/// user menu, F11 opens the plugin surface and F12 lists the screens.
#[test]
fn e2e_clicking_the_previously_dead_key_cells_runs_their_painted_actions() {
    let (left, right) = fixture(&[("a.txt", "a")]);

    // F2 covers columns 9..18 at this width.
    let user_menu = run_until(left.path(), right.path(), vec![click(12, SCREEN_H - 1)]);
    assert!(
        user_menu.contains("User menu") && user_menu.contains("Edit user menu"),
        "2UserMn must open the available command menu:\n{user_menu}"
    );

    // F11 covers 84..92 and F12 covers 92..100 at this width.
    let plugin = run_until(left.path(), right.path(), vec![click(86, SCREEN_H - 1)]);
    assert!(
        plugin.contains("Plugin commands") && plugin.contains("MCP status"),
        "11Plugin must open Far's plugin commands list:\n{plugin}"
    );

    let screens = run_until(left.path(), right.path(), vec![click(96, SCREEN_H - 1)]);
    assert!(
        screens.contains("Screens") && screens.contains("Console transcript"),
        "12Screen must offer the available screens:\n{screens}"
    );
}

/// Scrolling over either panel makes that panel active and moves its cursor,
/// so the next operation acts on exactly what the pointer was over.
#[test]
fn e2e_wheel_moves_the_cursor_in_the_panel_under_the_pointer() {
    let (left, right) = fixture(&[("from-left.txt", "left")]);
    fs::write(right.path().join("from-right.txt"), "right").expect("seed right file");

    let mut events = vec![
        scroll_down(SCREEN_W - 10, 4),
        key(KeyCode::F(5)),
        key(KeyCode::Enter),
    ];
    events.extend(quit());
    run(left.path(), right.path(), events);

    assert!(
        left.path().join("from-right.txt").exists(),
        "the wheel must activate the right panel and select its file"
    );
}

/// Overlays use the same wheel gesture: in the viewer it is the Down key, so
/// the first line leaves the viewport and the next one appears at the bottom.
#[test]
fn e2e_wheel_scrolls_the_open_viewer() {
    let body = (0..30)
        .map(|line| format!("viewer-line-{line:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (left, right) = fixture(&[("long.txt", &body)]);

    let screen = run_until(
        left.path(),
        right.path(),
        vec![
            key(KeyCode::Down),
            key(KeyCode::F(3)),
            scroll_down(SCREEN_W / 2, SCREEN_H / 2),
        ],
    );

    assert!(
        !screen.contains("viewer-line-00") && screen.contains("viewer-line-20"),
        "the wheel must scroll the viewer by one line:\n{screen}"
    );
}

/// …and the same for a destructive one, which must still show its dialog: the
/// click goes through the key, so it goes through the guard too.
#[test]
fn e2e_clicking_delete_still_asks_first() {
    let (left, right) = fixture(&[("guarded.txt", "x")]);

    // F8 is the eighth cell: columns 60..68 at 100 wide.
    let screen = run_until(
        left.path(),
        right.path(),
        vec![key(KeyCode::Down), click(62, SCREEN_H - 1)],
    );

    assert!(
        screen.contains("Do you wish to delete the file?"),
        "the mouse must not bypass the confirmation:\n{screen}"
    );
    assert!(
        left.path().join("guarded.txt").exists(),
        "and nothing is deleted yet"
    );
}
