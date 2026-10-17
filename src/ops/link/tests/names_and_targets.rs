//! Purpose: Where a typed name lands, failures, and relative targets.

use super::*;

/// Replace whatever `field` holds with `text`, the way the user would: the
/// prompt has keys, not setters.
fn retype(prompt: &mut NamePrompt, field: NameField, text: &str) {
    prompt.focus(field);
    let held = match field {
        NameField::Name => prompt.name().len(),
        NameField::Target => prompt.link_target().len(),
        _ => 0,
    };
    for _ in 0..held {
        prompt.backspace();
    }
    for ch in text.chars() {
        prompt.push(ch);
    }
}

/// A full path goes where it says, and the cursor follows it into whichever
/// panel lists that directory — here the active one.
#[cfg(unix)]
#[test]
fn a_full_name_in_the_active_panel_puts_the_cursor_there() {
    let (mut app, root) = two_dirs(&["notes.txt"]);
    let link = source_dir(&root).join("alias");
    let mut prompt = open_prompt(&mut app);
    retype(&mut prompt, NameField::Name, &link.display().to_string());
    app.close_overlay();

    create(&mut app, &prompt);

    assert!(link.is_symlink());
    assert_eq!(
        selected_name(&app, PanelSide::Left).as_deref(),
        Some("alias"),
        "the active panel's cursor moved to it"
    );
    assert_eq!(
        fs::read_dir(other_dir(&root)).expect("read").count(),
        0,
        "nothing went to the other panel"
    );
}

/// A blank name is not a name to create: the dialog closes quietly, as F7's
/// does.
#[test]
fn a_blank_name_makes_nothing() {
    let (mut app, root) = two_dirs(&["notes.txt"]);
    let mut prompt = open_prompt(&mut app);
    retype(&mut prompt, NameField::Name, "");
    app.close_overlay();

    create(&mut app, &prompt);

    assert_eq!(fs::read_dir(other_dir(&root)).expect("read").count(), 0);
}

/// A name without a directory is relative to the other panel — the only
/// reading under which "in the other panel by default" survives editing the
/// field down to a bare name.
#[cfg(unix)]
#[test]
fn a_bare_name_is_relative_to_the_other_panel() {
    let (mut app, root) = two_dirs(&["notes.txt"]);
    let mut prompt = open_prompt(&mut app);
    retype(&mut prompt, NameField::Name, "alias");
    app.close_overlay();

    create(&mut app, &prompt);

    assert!(other_dir(&root).join("alias").is_symlink());
}

#[test]
fn a_failure_is_reported_not_swallowed() {
    let (mut app, root) = two_dirs(&["notes.txt"]);
    let mut prompt = open_prompt(&mut app);
    choose_hard(&mut prompt);
    // A hard link to something that is not there cannot be made.
    let ghost = source_dir(&root).join("ghost");
    retype(&mut prompt, NameField::Target, &ghost.display().to_string());
    app.close_overlay();

    create(&mut app, &prompt);

    assert!(!other_dir(&root).join("notes.txt").exists());
    assert!(
        !status_text(&app).contains("Link"),
        "not a success: {}",
        status_text(&app)
    );
    assert!(!status_text(&app).is_empty());
}

#[test]
fn a_relative_target_is_judged_from_the_links_directory() {
    let (mut app, root) = two_dirs(&["photos/"]);
    let mut prompt = open_prompt(&mut app);
    retype(&mut prompt, NameField::Target, "../source/photos");
    choose_hard(&mut prompt);
    assert!(
        !is_possible(&other_dir(&root), &prompt),
        "the relative target is a directory"
    );
    retype(&mut prompt, NameField::Name, "alias");
    assert!(!is_possible(&other_dir(&root), &prompt));
    fs::create_dir(other_dir(&root).join("sub")).unwrap();
    retype(&mut prompt, NameField::Name, "sub/alias");
    retype(&mut prompt, NameField::Target, "../../source/photos");
    assert!(!is_possible(&other_dir(&root), &prompt));
}

#[test]
fn a_hard_link_to_a_relative_target_finds_it_beside_the_link() {
    let (mut app, root) = two_dirs(&["notes.txt"]);
    let mut prompt = open_prompt(&mut app);
    retype(&mut prompt, NameField::Target, "../source/notes.txt");
    retype(&mut prompt, NameField::Name, "alias");
    choose_hard(&mut prompt);
    app.close_overlay();
    create(&mut app, &prompt);
    let link = other_dir(&root).join("alias");
    assert!(!fs::symlink_metadata(&link)
        .expect("a hard link was made")
        .is_symlink());
    fs::write(source_dir(&root).join("notes.txt"), b"changed").unwrap();
    assert_eq!(fs::read(&link).expect("read"), b"changed");
    assert_eq!(
        selected_name(&app, PanelSide::Right).as_deref(),
        Some("alias")
    );
}

#[cfg(unix)]
#[test]
fn a_relative_target_is_kept_relative_and_resolves_from_the_link() {
    let (mut app, root) = two_dirs(&["notes.txt"]);
    let mut prompt = open_prompt(&mut app);
    retype(&mut prompt, NameField::Target, "../source/notes.txt");
    app.close_overlay();
    create(&mut app, &prompt);
    let link = other_dir(&root).join("notes.txt");
    assert_eq!(
        fs::read_link(&link).unwrap(),
        Path::new("../source/notes.txt")
    );
    assert_eq!(fs::read(&link).unwrap(), b"payload");
}
