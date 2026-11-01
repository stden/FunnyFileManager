//! Purpose: Space analysis acceptance steps: a git repository in the left panel,
//! the Commands row, the dialog's cursor, and the checks that nothing was removed.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use cucumber::{given, then, when};

use super::common::open_command_row;
use super::{BehaviorWorld, Case};
use crate::{
    menu::MenuCommand,
    models::{Overlay, PanelSide},
    sessions::SessionStore,
};

/// Presses that may move the dialog's cursor to any row it lists.
const MAX_ROW_PRESSES: usize = 100;

fn left_folder(case: &Case) -> PathBuf {
    case.root.path().join("left")
}

/// The files the repository fixture creates; no Space analysis key may remove one.
fn fixture_files(left: &Path) -> [PathBuf; 4] {
    [
        left.join(".gitignore"),
        left.join("tracked.txt"),
        left.join("notes.txt"),
        left.join("target").join("artifact.bin"),
    ]
}

fn git(folder: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(args)
        .status()
        .expect("git must be installed for the Space analysis scenarios");
    assert!(status.success(), "git {args:?} failed");
}

#[given("a git repository in the left panel with a tracked file, an ignored target folder and an untracked note")]
fn repository(world: &mut BehaviorWorld) {
    let case = world.case();
    let left = left_folder(case);
    git(&left, &["init", "--quiet"]);
    fs::write(left.join(".gitignore"), "target/\n").unwrap();
    fs::write(left.join("tracked.txt"), "tracked\n").unwrap();
    git(&left, &["add", ".gitignore", "tracked.txt"]);
    fs::create_dir(left.join("target")).unwrap();
    fs::write(left.join("target").join("artifact.bin"), [0_u8; 4096]).unwrap();
    fs::write(left.join("notes.txt"), "not in git\n").unwrap();
    crate::utils::refresh_panels(&mut case.app).unwrap();
}

#[given("the active panel shows a NetBox sessions list")]
fn netbox_panel(world: &mut BehaviorWorld) {
    let case = world.case();
    case.app.sessions = SessionStore::load_from(case.root.path().join("ffm.yml"));
    crate::test_support::netbox::open_empty_sessions(&mut case.app, PanelSide::Left);
}

#[when("I choose Space analysis from the F9 menu")]
fn choose(world: &mut BehaviorWorld) {
    open_command_row(world.case(), MenuCommand::SpaceAnalysis);
}

#[then("the Space analysis dialog is open")]
fn dialog_is_open(world: &mut BehaviorWorld) {
    assert!(
        matches!(world.case().app.overlay, Overlay::SpaceAnalysis(_)),
        "the Space analysis dialog must be open"
    );
}

#[then("the Space analysis dialog names the left folder")]
fn names_folder(world: &mut BehaviorWorld) {
    let case = world.case();
    let Overlay::SpaceAnalysis(dialog) = &case.app.overlay else {
        panic!("the Space analysis dialog must be open");
    };
    assert_eq!(
        dialog.folder, case.app.left_panel.path,
        "the dialog names another folder"
    );
    let row = format!("Folder: {}", crate::path_display::label(&dialog.folder));
    let screen = case.screen.text();
    assert!(
        screen.contains(&row),
        "no dialog row reads {row:?}\n{screen}"
    );
}

#[then(expr = "the Space analysis labels {string} as {string}")]
fn labels(world: &mut BehaviorWorld, name: String, label: String) {
    let screen = world.case().screen.text();
    let row = format!("{name}  {label}");
    assert!(screen.contains(&row), "no row reads {row:?}\n{screen}");
}

#[then(expr = "the Space analysis reclaims {string}")]
fn reclaims(world: &mut BehaviorWorld, size: String) {
    let screen = world.case().screen.text();
    let totals = format!("Reclaimable safely {size}");
    assert!(screen.contains(&totals), "missing {totals:?}\n{screen}");
}

#[when(expr = "I move the Space analysis cursor to {string}")]
fn move_cursor(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    for _ in 0..MAX_ROW_PRESSES {
        let Overlay::SpaceAnalysis(dialog) = &case.app.overlay else {
            panic!("the Space analysis dialog must be open");
        };
        if dialog.selected_name() == Some(name.as_str()) {
            return;
        }
        case.press("Down");
    }
    panic!("the Space analysis dialog has no row {name:?}");
}

#[then("every file of the repository fixture still exists")]
fn files_survive(world: &mut BehaviorWorld) {
    let left = left_folder(world.case());
    for path in fixture_files(&left) {
        assert!(path.is_file(), "removed: {}", path.display());
    }
    assert!(left.join(".git").is_dir(), "the git folder was removed");
}
