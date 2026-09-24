//! Purpose: cursor movement, paging, Enter on a directory and the go-to-root keys.

use super::*;

#[test]
fn filesystem_root_preserves_the_active_filesystem() {
    use std::path::Path;
    #[cfg(windows)]
    for (path, root) in [
        (r"D:\work\project", r"D:\"),
        (
            r"\\wsl.localhost\Ubuntu\home\user",
            r"\\wsl.localhost\Ubuntu\",
        ),
        (
            r"\\?\UNC\wsl.localhost\Debian\etc",
            r"\\?\UNC\wsl.localhost\Debian\",
        ),
    ] {
        assert_eq!(filesystem_root(Path::new(path)), Path::new(root));
    }
    #[cfg(not(windows))]
    assert_eq!(filesystem_root(Path::new("/home/user")), Path::new("/"));
}

#[test]
fn go_to_root_lands_on_the_root_and_drops_the_quick_search() {
    let (mut app, mut terminal, dir) = app_and_terminal(&["a.txt"]);
    let root = dir
        .path()
        .ancestors()
        .last()
        .expect("absolute fixture root");
    app.quick_search.push('a');

    dispatch_key(
        &mut terminal,
        &mut app,
        KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::CONTROL),
    );

    assert_eq!(app.get_active_panel_ref().path, root);
    assert_eq!(
        app.quick_search.query(),
        None,
        "a search typed in the old directory means nothing in the new one"
    );
}

/// Go to root is one of the ways out of a folder that was deleted under the
/// panel; it must not depend on that folder still being there.
#[test]
fn go_to_root_works_from_a_deleted_folder() {
    let (mut app, mut terminal, dir) = app_and_terminal(&[]);
    let inner = dir.path().join("gone").join("inner");
    fs::create_dir_all(&inner).expect("test setup");
    app.left_panel.navigate_to(inner).expect("test setup");
    fs::remove_dir_all(dir.path().join("gone")).expect("test setup");
    let root = dir
        .path()
        .ancestors()
        .last()
        .expect("absolute fixture root");

    dispatch_key(
        &mut terminal,
        &mut app,
        KeyEvent::new(KeyCode::Char('\\'), KeyModifiers::CONTROL),
    );

    assert_eq!(app.get_active_panel_ref().path, root);
}

// ─── arrow keys, paging and Enter reach the panel through apply_action ──────

#[test]
fn the_arrow_keys_page_keys_and_enter_move_through_the_panel() {
    let (mut app, mut terminal, dir) = app_and_terminal(&["a.txt", "b.txt", "c.txt"]);
    fs::create_dir(dir.path().join("subdir")).expect("test setup");
    app.left_panel.load_entries().expect("reload");
    app.left_panel.select(Some(0));

    press(&mut app, &mut terminal, KeyCode::Down);
    assert_eq!(app.left_panel.selected(), Some(1), "Down moves one row");
    press(&mut app, &mut terminal, KeyCode::Up);
    assert_eq!(app.left_panel.selected(), Some(0), "Up moves back");

    press(&mut app, &mut terminal, KeyCode::End);
    assert_eq!(
        app.left_panel.selected(),
        Some(app.left_panel.entries.len() - 1),
        "End jumps to the last row"
    );

    press(&mut app, &mut terminal, KeyCode::PageUp);
    press(&mut app, &mut terminal, KeyCode::PageDown);
    // A page on a listing this short still lands on a real row, not off the end.
    assert!(app.left_panel.selected().is_some());

    select_named(&mut app.left_panel, "subdir");
    let before = app.left_panel.path.clone();
    press(&mut app, &mut terminal, KeyCode::Enter);
    assert_ne!(app.left_panel.path, before, "Enter opened the directory");
}

/// A terminal without the kitty protocol sends Ctrl+\ as byte 0x1C, which
/// crossterm reports as Ctrl+4. Decoded for that terminal, it goes to the
/// root; it must never switch the panel to Wide view. (The Windows console
/// reports the key itself, so there is nothing to decode there.)
#[cfg(unix)]
#[test]
fn a_legacy_terminals_ctrl_backslash_goes_to_the_root_not_to_wide_view() {
    let (mut app, mut terminal, dir) = app_and_terminal(&["a.txt"]);
    let root = dir
        .path()
        .ancestors()
        .last()
        .expect("absolute fixture root");
    let view = app.get_active_panel_ref().view;
    let byte_0x1c = KeyEvent::new(KeyCode::Char('4'), KeyModifiers::CONTROL);

    let key = crate::terminal::KeyboardProtocol::Legacy.decode(byte_0x1c);
    dispatch_key(&mut terminal, &mut app, key);

    assert_eq!(app.get_active_panel_ref().view, view, "the view changed");
    assert_eq!(app.get_active_panel_ref().path, root);
}

/// With the kitty protocol Ctrl+4 is its own key, and keeps Far's meaning.
#[cfg(unix)]
#[test]
fn a_kitty_terminals_ctrl_4_is_wide_view() {
    let (mut app, mut terminal, dir) = app_and_terminal(&["a.txt"]);
    let ctrl_4 = KeyEvent::new(KeyCode::Char('4'), KeyModifiers::CONTROL);

    let key = crate::terminal::KeyboardProtocol::Kitty.decode(ctrl_4);
    dispatch_key(&mut terminal, &mut app, key);

    assert_eq!(app.get_active_panel_ref().view, models::PanelView::Wide);
    assert_eq!(app.get_active_panel_ref().path, dir.path());
}
