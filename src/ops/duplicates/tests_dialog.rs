//! Purpose: the Find duplicate blocks dialog: what it opens with, what Start
//! refuses, the list it opens, and the status line when a walk stopped early.

use std::time::{Duration, Instant};

use super::tests::tree;
use super::walk::Budget;
use super::{accept, open, run, Query};
use crate::mask::Mask;
use crate::models::{App, ChoiceTarget, Duplicates, MenuDialog, Overlay, Panel};
use crate::status::Status;

fn app_on(folder: &std::path::Path) -> App {
    let panel = Panel::new(folder.to_path_buf()).expect("panel");
    App::from_panels(panel.clone(), panel)
}

fn dialog(app: &mut App) -> &mut Duplicates {
    let Overlay::Duplicates(dialog) = &mut app.overlay else {
        panic!("the Find duplicate blocks dialog");
    };
    dialog.as_mut()
}

fn status_text(app: &App) -> &str {
    let Status::CommandOutput(text) = &app.status else {
        panic!("a command-output status");
    };
    text
}

fn budget(files: usize, deadline: Instant) -> Budget {
    Budget {
        files,
        bytes: u64::MAX,
        deadline,
    }
}

fn six_shared_lines() -> String {
    (1..=6)
        .map(|n| format!("shared line {n}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn open_starts_on_the_active_folder_with_the_defaults() {
    let folder = tree(&[]);
    let mut app = app_on(folder.path());
    open(&mut app);
    let dialog = dialog(&mut app);
    assert_eq!(dialog.folder.text(), folder.path().display().to_string());
    assert_eq!(dialog.mask.text(), "*");
    assert_eq!(dialog.min_block.text(), "6");
    assert_eq!(dialog.groups.text(), "50");
}

#[test]
fn start_refuses_a_block_under_three_lines_and_keeps_the_dialog_open() {
    let folder = tree(&[]);
    let mut app = app_on(folder.path());
    open(&mut app);
    dialog(&mut app).min_block.set_text("2");
    accept(&mut app);
    assert!(matches!(app.overlay, Overlay::Duplicates(_)));
    assert_eq!(
        status_text(&app),
        "Minimum block must be a number of lines, 3 or more."
    );
}

#[test]
fn start_refuses_a_folder_that_is_not_there() {
    let folder = tree(&[]);
    let mut app = app_on(folder.path());
    open(&mut app);
    let missing = folder.path().join("missing");
    dialog(&mut app)
        .folder
        .set_text(&missing.display().to_string());
    accept(&mut app);
    assert!(matches!(app.overlay, Overlay::Duplicates(_)));
    assert!(status_text(&app).starts_with("Not a folder: "));
}

#[test]
fn start_lists_each_copy_and_each_row_opens_its_first_line() {
    let block = six_shared_lines();
    let folder = tree(&[("a.rs", &block), ("b.rs", &block)]);
    let mut app = app_on(folder.path());
    open(&mut app);
    accept(&mut app);
    let Overlay::MenuDialog(list) = &app.overlay else {
        panic!("the list of repeats");
    };
    let MenuDialog::Choices { title, rows, .. } = list.as_ref() else {
        panic!("choices");
    };
    assert_eq!(title, "Find duplicate blocks");
    assert_eq!(rows.len(), 2, "one row per copy");
    assert!(rows[0].label.contains("2x 6 lines"), "{}", rows[0].label);
    assert!(rows[0].label.ends_with("a.rs:1-6"), "{}", rows[0].label);
    let ChoiceTarget::FileLine { path, line } = &rows[1].target else {
        panic!("a row that opens a line");
    };
    assert_eq!(
        (path.as_path(), *line),
        (folder.path().join("b.rs").as_path(), 1)
    );
}

#[test]
fn a_walk_that_stopped_early_says_which_limit_did_it() {
    let folder = tree(&[("a.rs", "one"), ("b.rs", "two")]);
    let mut app = app_on(folder.path());
    let query = Query {
        folder: folder.path().to_path_buf(),
        mask: Mask::parse("*"),
        min_block: 3,
        groups: 5,
    };
    run(
        &mut app,
        &query,
        budget(1, Instant::now() + Duration::from_secs(60)),
    );
    assert_eq!(
        status_text(&app),
        "Duplicate search stopped at the file limit. The list is incomplete."
    );
    run(&mut app, &query, budget(usize::MAX, Instant::now()));
    assert_eq!(
        status_text(&app),
        "Duplicate search stopped at the time limit. The list is incomplete."
    );
}

#[test]
fn an_empty_mask_compares_every_file() {
    let block = six_shared_lines();
    let folder = tree(&[("a.rs", &block), ("b.txt", &block)]);
    let mut app = app_on(folder.path());
    open(&mut app);
    dialog(&mut app).mask.set_text("");
    accept(&mut app);
    let Overlay::MenuDialog(list) = &app.overlay else {
        panic!("the list of repeats");
    };
    let MenuDialog::Choices { rows, .. } = list.as_ref() else {
        panic!("choices");
    };
    assert_eq!(rows.len(), 2, "the .txt copy counts with the .rs one");
}

#[test]
fn start_refuses_a_group_count_that_is_not_a_number() {
    let folder = tree(&[]);
    let mut app = app_on(folder.path());
    open(&mut app);
    dialog(&mut app).groups.set_text("zero");
    accept(&mut app);
    assert!(matches!(app.overlay, Overlay::Duplicates(_)));
    assert_eq!(status_text(&app), "Groups must be a number, 1 or more.");
}
