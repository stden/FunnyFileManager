//! Purpose: Creating symbolic and hard links, and names that are already taken.

use super::*;

// ─── creating ───────────────────────────────────────────────────────────────

#[cfg(unix)]
#[test]
fn a_symbolic_link_lands_in_the_other_panel_with_the_cursor_on_it() {
    let (mut app, root) = two_dirs(&["notes.txt"]);
    let prompt = open_prompt(&mut app);
    app.close_overlay();

    create(&mut app, &prompt);

    let link = other_dir(&root).join("notes.txt");
    let target = source_dir(&root).join("notes.txt");
    assert_eq!(
        fs::read_link(&link).expect("a symbolic link was made"),
        target
    );
    assert_eq!(
        selected_name(&app, PanelSide::Right).as_deref(),
        Some("notes.txt"),
        "the cursor finds it"
    );
    assert_eq!(
        app.status,
        Status::Linked { link, target },
        "the status line names both ends"
    );
}

/// The default for a folder: Enter on the untouched dialog, and the other
/// panel lists a folder link that opens like the folder itself.
#[cfg(unix)]
#[test]
fn a_symbolic_link_to_a_folder_is_a_folder_in_the_other_panel() {
    let (mut app, root) = two_dirs(&["photos/"]);
    fs::write(source_dir(&root).join("photos").join("one.jpg"), b"jpeg").expect("test setup");
    let prompt = open_prompt(&mut app);
    app.close_overlay();

    create(&mut app, &prompt);

    let link = other_dir(&root).join("photos");
    assert!(link.is_symlink(), "a link, not a copy");
    assert!(link.join("one.jpg").is_file(), "and it opens as the folder");
    assert_eq!(
        selected_name(&app, PanelSide::Right).as_deref(),
        Some("photos"),
        "the cursor finds the new folder link"
    );
}

/// A link whose target is itself a link to a folder still leads to a folder.
#[cfg(unix)]
#[test]
fn a_link_to_a_folder_link_still_opens_as_a_folder() {
    let (mut app, root) = two_dirs(&["photos/"]);
    std::os::unix::fs::symlink("photos", source_dir(&root).join("shots")).expect("test setup");
    app.left_panel.load_entries().expect("reload");
    app.left_panel.select_named("shots");
    let prompt = open_prompt(&mut app);
    app.close_overlay();

    create(&mut app, &prompt);

    let link = other_dir(&root).join("shots");
    assert!(link.is_symlink());
    assert!(link.is_dir(), "two hops, still a folder");
}

#[test]
fn a_hard_link_is_the_same_file_under_a_second_name() {
    let (mut app, root) = two_dirs(&["notes.txt"]);
    let mut prompt = open_prompt(&mut app);
    choose_hard(&mut prompt);
    app.close_overlay();

    create(&mut app, &prompt);

    let link = other_dir(&root).join("notes.txt");
    fs::write(source_dir(&root).join("notes.txt"), b"changed").expect("write");
    assert_eq!(
        fs::read(&link).expect("a hard link was made"),
        b"changed",
        "writing through one name is seen through the other"
    );
    assert!(
        !fs::symlink_metadata(&link).expect("metadata").is_symlink(),
        "a hard link, not a symbolic one"
    );
}

#[test]
fn a_hard_link_to_a_folder_is_refused_before_the_kernel_gets_to() {
    let (mut app, root) = two_dirs(&["photos/"]);
    let mut prompt = open_prompt(&mut app);
    choose_hard(&mut prompt);
    app.close_overlay();

    create(&mut app, &prompt);

    assert!(!other_dir(&root).join("photos").exists());
    assert!(
        status_text(&app).contains("cannot be hard-linked"),
        "{}",
        status_text(&app)
    );
}

#[test]
fn an_existing_name_is_refused_rather_than_replaced() {
    let (mut app, root) = two_dirs(&["notes.txt"]);
    let taken = other_dir(&root).join("notes.txt");
    fs::write(&taken, b"mine").expect("test setup");
    let prompt = open_prompt(&mut app);
    app.close_overlay();

    create(&mut app, &prompt);

    assert_eq!(fs::read(&taken).expect("still there"), b"mine");
    assert_eq!(app.status, Status::AlreadyExists(taken));
}

/// A dangling link is a name that is taken even though nothing is behind it:
/// `exists` would call it free and the new link would go in over it.
#[cfg(unix)]
#[test]
fn a_dangling_link_at_the_name_is_still_a_taken_name() {
    let (mut app, root) = two_dirs(&["notes.txt"]);
    let taken = other_dir(&root).join("notes.txt");
    std::os::unix::fs::symlink("ghost", &taken).expect("test setup");
    let prompt = open_prompt(&mut app);
    app.close_overlay();

    create(&mut app, &prompt);

    assert_eq!(
        fs::read_link(&taken).expect("still the old link"),
        Path::new("ghost")
    );
    assert_eq!(app.status, Status::AlreadyExists(taken));
}
