//! Purpose: Opening the link prompt, and which kinds of link its drop-down may offer.

use super::*;

// ─── opening ────────────────────────────────────────────────────────────────

/// Far pre-fills the link name with the passive panel's directory: that is
/// where a link goes unless the user says otherwise, and showing the path is
/// what lets them say otherwise.
#[test]
fn the_prompt_names_the_link_in_the_other_panel_and_targets_the_cursor_entry() {
    let (mut app, root) = two_dirs(&["notes.txt"]);
    let prompt = open_prompt(&mut app);

    assert_eq!(prompt.purpose(), crate::models::NamePurpose::Link);
    assert_eq!(
        prompt.link(),
        LinkType::Symbolic,
        "the kind that works for everything"
    );
    assert_eq!(
        Path::new(prompt.name()),
        other_dir(&root).join("notes.txt"),
        "the link is named in the other panel by default"
    );
    assert_eq!(
        Path::new(prompt.link_target()),
        source_dir(&root).join("notes.txt"),
        "the target is the entry under the cursor, absolute"
    );
}

#[test]
fn nothing_to_link_when_the_cursor_is_on_the_parent_row() {
    let (mut app, _root) = two_dirs(&[]);
    open(&mut app);
    assert!(!app.overlay.is_active(), "no dialog over an empty listing");
    assert_eq!(status_text(&app), "Nothing to link");
}

// ─── what the drop-down may offer ───────────────────────────────────────────

#[test]
fn a_hard_link_is_possible_for_a_file_but_not_for_a_folder() {
    let (mut app, root) = two_dirs(&["notes.txt", "photos/"]);
    let mut prompt = open_prompt(&mut app);
    assert!(is_possible(&other_dir(&root), &prompt), "symbolic, file");
    choose_hard(&mut prompt);
    assert!(is_possible(&other_dir(&root), &prompt), "hard, file");

    app.close_overlay();
    app.left_panel.select_named("photos");
    let mut prompt = open_prompt(&mut app);
    assert!(is_possible(&other_dir(&root), &prompt), "symbolic, folder");
    choose_hard(&mut prompt);
    assert!(
        !is_possible(&other_dir(&root), &prompt),
        "no kernel hard-links a directory"
    );
}
