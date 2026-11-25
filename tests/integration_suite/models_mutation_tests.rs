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

mod app_and_values;
mod panel_navigation;
mod selection_and_marks;
mod sorting;

fn select_entry(panel: &mut Panel, name: &str) {
    let index = panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .expect("entry not found");
    panel.select(Some(index));
}
