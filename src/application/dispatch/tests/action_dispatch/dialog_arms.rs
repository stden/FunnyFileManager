//! Purpose: the arms that open a dialog or report why not: rename, transfer, menu, info.

use super::*;

// ─── the arms that report why they did nothing ──────────────────────────────

#[test]
fn renaming_the_parent_row_says_there_is_nothing_to_rename() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    select_named(&mut app.left_panel, "..");

    dispatch_key(
        &mut terminal,
        &mut app,
        KeyEvent::new(KeyCode::F(6), KeyModifiers::SHIFT),
    );

    assert!(!app.overlay.is_active(), "no dialog for the parent row");
    assert!(
        app.status.to_string().contains("Nothing to rename"),
        "{}",
        app.status
    );
}

/// F5 with an empty listing has nothing to copy, and must close rather than
/// put up a dialog about no files.
#[test]
fn the_transfer_dialog_does_not_open_with_nothing_to_transfer() {
    let dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    // A panel at the filesystem root has no `..` row and, here, no entries the
    // action can take — which is the empty case the guard is for.
    app.left_panel = Panel::from_entries(dir.path().to_path_buf(), Vec::new());
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("test setup");

    press(&mut app, &mut terminal, KeyCode::F(5));

    assert!(!app.overlay.is_active(), "nothing to copy, so no dialog");
}

// ─── the menu opens on the side you are working in ──────────────────────────

#[test]
fn f9_opens_the_menu_on_whichever_panel_is_active() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);

    press(&mut app, &mut terminal, KeyCode::F(9));
    let Overlay::Menu(menu) = &app.overlay else {
        panic!("the bar is open");
    };
    assert_eq!(menu.heading, menu::Heading::Left);
    press(&mut app, &mut terminal, KeyCode::F(9));

    app.active_panel = PanelSide::Right;
    press(&mut app, &mut terminal, KeyCode::F(9));
    let Overlay::Menu(menu) = &app.overlay else {
        panic!("the bar is open");
    };
    assert_eq!(
        menu.heading,
        menu::Heading::Right,
        "the panel menus act on their own side, so it opens on the active one"
    );
}

// ─── Ctrl+L measures the active panel ───────────────────────────────────────

#[test]
fn ctrl_l_opens_the_information_overlay_for_the_active_panel() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt", "b.txt"]);

    dispatch_key(
        &mut terminal,
        &mut app,
        KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
    );

    let Overlay::Information(stats) = &app.overlay else {
        panic!("Ctrl+L opens the information overlay");
    };
    assert_eq!(stats.files, 2, "it measured the active panel's directory");
}

/// The panel menu's "Info panel" row carries the same `Ctrl+L` shortcut, and
/// runs it through the same dispatcher a keypress would — not a second,
/// menu-only implementation of "show me what's here".
#[test]
fn selecting_info_panel_from_the_menu_runs_it_through_the_dispatcher() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);

    press(&mut app, &mut terminal, KeyCode::F(9));
    assert!(matches!(app.overlay, Overlay::Menu(_)), "F9 opened the bar");

    // "&Info panel" is the row whose hotkey is 'i'; picking it hands the
    // dispatcher its Ctrl+L shortcut once the menu has closed.
    press(&mut app, &mut terminal, KeyCode::Char('i'));

    let Overlay::Information(_) = &app.overlay else {
        panic!("the menu row ran Ctrl+L through the normal dispatcher");
    };
}

// ─── Shift+F6 on a real entry opens the rename dialog ───────────────────────

#[test]
fn shift_f6_on_a_real_entry_opens_the_rename_dialog_prefilled() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["report.md"]);
    select_named(&mut app.left_panel, "report.md");

    dispatch_key(
        &mut terminal,
        &mut app,
        KeyEvent::new(KeyCode::F(6), KeyModifiers::SHIFT),
    );

    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("Shift+F6 on a real entry opens the rename dialog");
    };
    assert_eq!(
        prompt.purpose(),
        models::NamePurpose::Rename,
        "not the make-folder dialog"
    );
    assert_eq!(prompt.original(), "report.md");
    assert_eq!(
        prompt.name(),
        "report.md",
        "the field starts prefilled with the current name"
    );
}

// ─── F5 counts more than one marked file ────────────────────────────────────

#[test]
fn the_transfer_dialog_counts_more_than_one_marked_file() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt", "b.txt"]);
    select_named(&mut app.left_panel, "a.txt");

    press(&mut app, &mut terminal, KeyCode::Insert); // marks a.txt, steps on
    press(&mut app, &mut terminal, KeyCode::Insert); // marks whatever is next
    assert_eq!(app.left_panel.marked.len(), 2, "both files ended up marked");

    press(&mut app, &mut terminal, KeyCode::F(5));

    let Overlay::CopyPrompt(prompt) = &app.overlay else {
        panic!("F5 with files marked opens the transfer dialog");
    };
    assert_eq!(prompt.what, "2 files");
}
