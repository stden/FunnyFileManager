//! Purpose: Strengthens model assertions against mutation-test survivors.
//!

use funny_file_manager::models::EntryKind;
use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::Modified;
use funny_file_manager::models::{App, DirEntry, Panel, PanelSide, SortMode, Viewer};
use funny_file_manager::scroll::ScrollTop;
use std::fs::{self, File};
use std::io::{self, Write};
use tempfile::TempDir;

#[path = "models_mutation_tests/app_and_values.rs"]
mod app_and_values;
#[path = "models_mutation_tests/panel_navigation.rs"]
mod panel_navigation;
#[path = "models_mutation_tests/selection_and_marks.rs"]
mod selection_and_marks;
#[path = "models_mutation_tests/sorting.rs"]
mod sorting;

#[expect(
    clippy::expect_used,
    reason = "test fixture: a panel without the named entry fails the test that chose it"
)]
fn select_entry(panel: &mut Panel, name: &str) {
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("entry not found");
    panel.select(Some(index));
}
