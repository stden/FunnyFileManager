//! Unit tests: what Alt+F6 asks, and what it makes on disk.
//!
//! The dialog is opened over real temporary directories and the link is read
//! back with `read_link` or compared by inode, because a test that only
//! looks at the prompt's fields would pass with a `create` that did nothing.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::{create, is_possible, open};
use crate::models::{App, LinkType, NameField, NamePrompt, Overlay, Panel, PanelSide};
use crate::status::Status;

/// An app whose active (left) panel lists `source`, with the right panel on
/// `other`: the two directories Alt+F6 works between.
fn app_between(source: &Path, other: &Path) -> App {
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(source.to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(other.to_path_buf()).expect("test setup");
    app.active_panel = PanelSide::Left;
    app
}

/// Two sibling directories, `source` holding `entries` (a trailing `/` makes a
/// directory), the cursor on the first of them.
fn two_dirs(entries: &[&str]) -> (App, TempDir) {
    let root = TempDir::new().expect("test setup");
    let source = root.path().join("source");
    let other = root.path().join("other");
    fs::create_dir_all(&source).expect("test setup");
    fs::create_dir_all(&other).expect("test setup");
    for entry in entries {
        match entry.strip_suffix('/') {
            Some(dir) => fs::create_dir(source.join(dir)).expect("test setup"),
            None => fs::write(source.join(entry), b"payload").expect("test setup"),
        }
    }
    let mut app = app_between(&source, &other);
    if let Some(first) = entries.first() {
        app.left_panel.select_named(first.trim_end_matches('/'));
    }
    (app, root)
}

fn other_dir(root: &TempDir) -> PathBuf {
    root.path().join("other")
}

fn source_dir(root: &TempDir) -> PathBuf {
    root.path().join("source")
}

fn open_prompt(app: &mut App) -> NamePrompt {
    open(app);
    let Overlay::NamePrompt(prompt) = &app.overlay else {
        panic!("Alt+F6 opens the name prompt");
    };
    prompt.clone()
}

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

fn choose_hard(prompt: &mut NamePrompt) {
    prompt.focus(NameField::Link);
    prompt.cycle(true);
    assert_eq!(prompt.link(), LinkType::Hard);
}

fn selected_name(app: &App, side: PanelSide) -> Option<String> {
    app.panel(side)
        .get_selected_entry()
        .map(|entry| entry.name.clone())
}

fn status_text(app: &App) -> String {
    match &app.status {
        Status::CommandOutput(text) => text.clone(),
        other => other.to_string(),
    }
}

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
        status_text(&app),
        format!("Link {} -> {}", link.display(), target.display()),
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
    assert!(
        status_text(&app).contains("already exists"),
        "{}",
        status_text(&app)
    );
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
    assert!(
        status_text(&app).contains("already exists"),
        "{}",
        status_text(&app)
    );
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
