//! Purpose: Public automation performs real panel, editor and confirmed file workflows.

use super::*;

#[test]
fn panel_navigation_and_f4_editing_save_real_file_bytes() {
    let root = workspace();
    let mut session = Session::start(root.path());
    let state = session.enter("left");
    assert_path(&state["panels"]["left"]["path"], &root.path().join("left"));
    session.select("file.txt");
    let state = session.keys(&["F4"]);
    assert_eq!(state["modal"]["context"], "editor");
    assert!(screen(&state).contains("alpha"));
    session.keys(&["Ctrl+Home"]);
    session.text("prefix ");
    session.action("editor", "save");
    assert_eq!(
        fs::read(root.path().join("left/file.txt")).unwrap(),
        b"prefix alpha\n"
    );
    let state = session.action("editor", "close");
    assert_eq!(state["modal"]["kind"], "none");
    session.close();
}

#[test]
fn named_mkdir_can_be_cancelled_and_then_confirmed() {
    let root = workspace();
    let mut session = Session::start(root.path());
    session.enter("left");
    let state = session.action("panels", "mkdir");
    assert_eq!(state["modal"]["kind"], "name_prompt");
    session.text("cancelled");
    session.keys(&["Esc"]);
    assert!(!root.path().join("left/cancelled").exists());
    session.action("panels", "mkdir");
    session.text("created");
    let state = session.keys(&["Enter"]);
    assert!(root.path().join("left/created").is_dir());
    assert!(state["panels"]["left"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["name"] == "created" && entry["is_dir"] == true));
    session.close();
}

#[test]
fn named_copy_requires_accepting_the_real_destination_dialog() {
    let root = workspace();
    let mut session = Session::start(root.path());
    session.enter("left");
    session.keys(&["Tab"]);
    session.enter("right");
    session.keys(&["Tab"]);
    session.select("file.txt");
    let state = session.action("panels", "copy");
    assert_eq!(state["modal"]["kind"], "copy_prompt");
    assert!(!root.path().join("right/file.txt").exists());
    session.keys(&["Esc"]);
    assert!(!root.path().join("right/file.txt").exists());
    session.action("panels", "copy");
    let state = session.keys(&["Enter"]);
    assert_eq!(state["modal"]["kind"], "none");
    assert_eq!(
        fs::read(root.path().join("right/file.txt")).unwrap(),
        b"alpha\n"
    );
    assert_eq!(
        fs::read(root.path().join("left/file.txt")).unwrap(),
        b"alpha\n"
    );
    session.close();
}

#[test]
fn named_delete_retains_the_file_until_an_explicit_confirmation() {
    let root = workspace();
    let mut session = Session::start(root.path());
    session.enter("left");
    session.select("file.txt");
    let state = session.action("panels", "delete");
    assert_eq!(state["modal"]["kind"], "delete_confirm");
    assert_eq!(
        fs::read(root.path().join("left/file.txt")).unwrap(),
        b"alpha\n"
    );
    session.keys(&["Esc"]);
    assert!(root.path().join("left/file.txt").exists());
    session.action("panels", "delete");
    let state = session.keys(&["Y"]);
    assert_eq!(state["modal"]["kind"], "none");
    assert!(!root.path().join("left/file.txt").exists());
    assert!(root.path().join("left").is_dir());
    session.close();
}
