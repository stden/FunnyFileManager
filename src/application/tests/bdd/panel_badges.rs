//! Purpose: BDD steps for script executable styling (`*.sh`, `*.py`, `*.cmd`) and dirty-repo badges.

use std::fs;

use cucumber::{gherkin::Step, given, then, when};
use ratatui::style::Color;

use super::BehaviorWorld;
use crate::models::FileName;

const EXECUTABLE_GREEN: Color = Color::Rgb(80, 250, 123);
const FILE_CYAN: Color = Color::Rgb(85, 221, 238);
const DIRTY_AMBER: Color = Color::Rgb(255, 190, 130);
const DIRECTORY_WHITE: Color = Color::Rgb(255, 255, 255);

#[given("the left panel contains files:")]
fn populate_left_files(world: &mut BehaviorWorld, step: &Step) {
    let case = world.case();
    let left = case.root.path().join("left");
    for row in &step.table().expect("table of file names").rows {
        fs::write(left.join(&row[0]), b"echo ok\n").unwrap();
    }
    case.app.left_panel.load_entries().unwrap();
    case.paint();
}

#[given("the left panel contains subdirectories:")]
fn populate_left_subdirs(world: &mut BehaviorWorld, step: &Step) {
    let case = world.case();
    let left = case.root.path().join("left");
    for row in &step.table().expect("table of folder names").rows {
        fs::create_dir_all(left.join(&row[0])).unwrap();
    }
    case.app.left_panel.load_entries().unwrap();
    case.paint();
}

#[then(expr = "panel entry {string} is marked executable and rendered in executable green")]
fn verify_executable_entry(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    let entry = case
        .app
        .left_panel
        .entries
        .iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("missing left entry {name:?}"));
    assert!(entry.executable, "{name:?} must be marked executable");
    let stem = name.split('.').next().unwrap_or(name.as_str());
    assert_eq!(case.screen.fg_of_text(stem), Some(EXECUTABLE_GREEN));
}

#[then(expr = "panel entry {string} is rendered in ordinary file cyan")]
fn verify_ordinary_file(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    let entry = case
        .app
        .left_panel
        .entries
        .iter()
        .find(|entry| entry.name == name)
        .unwrap();
    assert!(!entry.executable, "{name:?} must not be marked executable");
    let stem = name.split('.').next().unwrap_or(name.as_str());
    assert_eq!(case.screen.fg_of_text(stem), Some(FILE_CYAN));
}

#[when(expr = "background Git probe marks left folder {string} as dirty")]
fn mark_folder_dirty(world: &mut BehaviorWorld, name: String) {
    apply_probe(world, &name, true);
}

#[when(expr = "background Git probe marks left folder {string} as clean")]
fn mark_folder_clean(world: &mut BehaviorWorld, name: String) {
    apply_probe(world, &name, false);
}

fn apply_probe(world: &mut BehaviorWorld, name: &str, dirty: bool) {
    let case = world.case();
    let parent = case.app.left_panel.path.clone();
    let changed = case
        .app
        .left_panel
        .apply_dirty_probe(&parent, &FileName::new(name), dirty);
    assert!(
        changed,
        "probe for listed folder {name:?} must update state"
    );
    case.paint();
}

#[then(expr = "panel folder {string} is rendered in dirty-repo amber")]
fn verify_dirty_folder(world: &mut BehaviorWorld, name: String) {
    assert_eq!(world.case().screen.fg_of_text(&name), Some(DIRTY_AMBER));
}

#[then(expr = "panel folder {string} is rendered in directory white")]
fn verify_clean_folder(world: &mut BehaviorWorld, name: String) {
    assert_eq!(world.case().screen.fg_of_text(&name), Some(DIRECTORY_WHITE));
}

#[then(expr = "background Git probe for unlisted left folder {string} is ignored")]
fn verify_unlisted_probe_ignored(world: &mut BehaviorWorld, name: String) {
    let case = world.case();
    let parent = case.app.left_panel.path.clone();
    let changed = case
        .app
        .left_panel
        .apply_dirty_probe(&parent, &FileName::new(&name), true);
    assert!(!changed, "unlisted folder {name:?} must be ignored");
}
