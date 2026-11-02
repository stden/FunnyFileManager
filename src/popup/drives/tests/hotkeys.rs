//! Purpose: a row's own key in the drive menu — a drive letter or a plugin hotkey — opens it without the cursor, in either case, and a key no row answers to leaves the menu open.

use super::*;

/// Quick-select by letter opens `C:\`, `D:\`, … — paths that never exist on Unix
/// and need not exist on Windows either. Without the existence guard the panel
/// would jump to an unreadable path and the menu would close; instead the key is
/// swallowed and nothing moves.
#[test]
fn drive_menu_letter_key_does_nothing_when_that_drive_does_not_exist() {
    // `C:\` is real on Windows, so the key pressed has to be a letter this
    // machine has no drive for. On Unix the first candidate always qualifies.
    let absent = ('a'..='z')
        .find(|letter| !PathBuf::from(format!("{}:\\", letter.to_ascii_uppercase())).exists())
        .expect("no machine has all 26 drive letters mounted");

    let temp = TempDir::new().expect("temp dir");
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    open_drive_menu(&mut app, 0);
    let status_before = app.status_text();

    assert!(process_popup_input(&mut app, key(KeyCode::Char(absent))));

    assert_eq!(app.left_panel.path, temp.path());
    assert_eq!(app.status_text(), status_before);
    assert_eq!(drive_index(&app), 0, "the menu stays open on its selection");
}

/// Letter quick-select opens the drive when that path exists (Windows). Pins
/// the `open_drive` success path that Unix never reaches because `C:\` is not
/// a real path there.
#[cfg(windows)]
#[test]
fn drive_menu_letter_opens_an_existing_local_drive() {
    let mut app = App::new().expect("test app");
    open_drive_menu(&mut app, 0);

    // System drive is almost always C: on Windows CI images.
    assert!(process_popup_input(&mut app, key(KeyCode::Char('c'))));
    assert!(
        !app.is_overlay_active(),
        "choosing a real drive letter closes the menu"
    );
    assert!(
        app.left_panel.path.to_string_lossy().starts_with("C:"),
        "panel should sit on the chosen drive, got {}",
        app.left_panel.path.display()
    );
}

/// A key no row answers to is swallowed. This used to build `Q:\` and report
/// that the drive could not be opened, on a machine with no drive letters at
/// all.
#[test]
fn a_key_no_row_answers_to_leaves_the_menu_open() {
    let mut app = App::new().expect("test app");
    open_drive_menu(&mut app, 0);
    let before = app.left_panel.path.clone();

    process_popup_input(&mut app, key(KeyCode::Char('Q')));

    assert_eq!(app.left_panel.path, before);
    assert!(
        matches!(app.overlay, Overlay::DriveSelection { .. }),
        "an unknown key must be a harmless keypress"
    );
}

/// The row's own key opens it, exactly as Enter on that row would: Far offers
/// the drive letter and the digit beside a plugin row as shortcuts.
#[test]
fn a_rows_own_key_opens_it_without_the_cursor() {
    let mut app = App::new().expect("test app");
    open_drive_menu(&mut app, 0);
    let rows = crate::models::drive_rows();
    let first = rows.first().expect("a machine always has one filesystem");
    let shortcut = first.key.expect("a disk row always has a key");

    process_popup_input(&mut app, key(KeyCode::Char(shortcut)));

    assert!(matches!(app.overlay, Overlay::None), "the menu closes");
    assert!(
        app.status_text().contains("Changed to"),
        "{}",
        app.status_text()
    );
}

/// The lookup runs over the rows stored when the menu opened and ignores
/// case. Pinned with an injected row keyed `z`: the live key on the machines
/// the suite runs on is `/`, whose case swap is the identity, so a lookup
/// through the real drive list never leaves the equal-case path.
#[test]
fn the_key_lookup_ignores_case() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::DriveSelection {
        side: PanelSide::Left,
        selected_index: 0,
        rows: vec![crate::models::DriveRow {
            key: Some('z'),
            kind: crate::models::RowKind::Disk {
                label: "z-drive".to_owned(),
                device: crate::diskfree::DriveKind::Fixed,
                space: None,
            },
            entry: DriveEntry::Local(temp.path().to_path_buf()),
        }],
    });

    assert!(process_popup_input(&mut app, key(KeyCode::Char('Z'))));

    assert!(matches!(app.overlay, Overlay::None), "the menu closes");
    assert_eq!(
        app.left_panel.path,
        temp.path(),
        "`Z` must reach the row keyed `z`"
    );
}
