//! Purpose: Shift+F2 opens on the other panel and the archive under the cursor, or says why not.

use super::*;

#[test]
fn open_offers_the_other_panel_and_the_archive_under_the_cursor() {
    let temp = TempDir::new().expect("temp");
    let (left, right) = (temp.path().join("l"), temp.path().join("r"));
    fs::create_dir_all(&left).expect("left");
    fs::create_dir_all(&right).expect("right");
    let archive = pack(&left, "a.tar.gz");
    let mut app = app_between(&left, &right);
    select(&mut app, "a.tar.gz");

    open_extract(&mut app);
    let Overlay::ExtractPrompt(prompt) = &app.overlay else {
        panic!("dialog opens");
    };
    assert_eq!(prompt.archives, vec![archive]);
    assert_eq!(prompt.destination, right.to_string_lossy());
    assert!(prompt.can_extract());
}

#[test]
fn open_on_a_plain_file_says_so_and_offers_no_extract() {
    let temp = TempDir::new().expect("temp");
    fs::write(temp.path().join("notes.txt"), "x").expect("file");
    let mut app = app_between(temp.path(), temp.path());
    select(&mut app, "notes.txt");
    open_extract(&mut app);
    let Overlay::ExtractPrompt(prompt) = &app.overlay else {
        panic!("dialog opens");
    };
    assert!(prompt.archives.is_empty());
    assert!(!prompt.can_extract());
}
