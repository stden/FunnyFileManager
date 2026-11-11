//! Purpose: What the preview shows: folder totals, UTF-16, binaries, oversized and vanished files.

use super::*;

#[test]
fn a_folder_shows_its_totals() {
    let dir = fixture();
    let mut app = app_in(&dir);
    select(&mut app, "nested");
    toggle(&mut app);

    let Preview::Folder(stats) = preview(&app, PanelSide::Right) else {
        panic!("expected folder totals");
    };
    assert_eq!(stats.files, 2);
    assert_eq!(stats.directories, 0);
    assert_eq!(stats.total.bytes(), 12);
}

/// The byte-order mark is checked before the NUL heuristic: UTF-16 text is
/// previewed, as F3 and F4 show it, not declined as binary.
#[test]
fn a_utf16_file_with_a_bom_is_previewed_not_declined_as_binary() {
    let dir = fixture();
    fs::write(dir.path().join("wide.txt"), b"\xff\xfeH\x00i\x00").expect("write");
    let mut app = app_in(&dir);
    select(&mut app, "wide.txt");
    toggle(&mut app);

    let Preview::Text(text) = preview(&app, PanelSide::Right) else {
        panic!("expected a text preview");
    };
    assert_eq!(text.lines, vec!["Hi"]);
    assert_eq!(text.encoding.label(), "UTF-16 LE");
}

#[test]
fn a_binary_file_and_the_parent_row_are_declined_gracefully() {
    let dir = fixture();
    let mut app = app_in(&dir);
    select(&mut app, "blob.bin");
    toggle(&mut app);
    assert_eq!(
        preview(&app, PanelSide::Right),
        &Preview::Refused(Refusal::Binary)
    );

    select(&mut app, "..");
    refresh(&mut app);
    assert_eq!(preview(&app, PanelSide::Right), &Preview::Nothing);
}

#[test]
fn a_file_over_the_limit_is_not_read() {
    let dir = fixture();
    let huge = dir.path().join("huge.log");
    let file = fs::File::create(&huge).expect("create");
    file.set_len(crate::utils::MAX_FILE_SIZE.bytes() + 1)
        .expect("sparse file");
    let mut app = app_in(&dir);
    select(&mut app, "huge.log");

    toggle(&mut app);

    assert!(
        matches!(
            preview(&app, PanelSide::Right),
            Preview::Refused(Refusal::TooLarge(_))
        ),
        "{:?}",
        preview(&app, PanelSide::Right)
    );
}

#[test]
fn a_file_that_vanished_is_reported_not_crashed() {
    let dir = fixture();
    let mut app = app_in(&dir);
    select(&mut app, "alpha.txt");
    fs::remove_file(dir.path().join("alpha.txt")).expect("remove");

    toggle(&mut app);

    assert!(matches!(
        preview(&app, PanelSide::Right),
        Preview::Refused(Refusal::Unreadable(_))
    ));
}
