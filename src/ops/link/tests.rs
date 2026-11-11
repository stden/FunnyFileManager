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

mod link_creation;
mod link_prompt;
mod names_and_targets;
