//! Unit tests: Ctrl+Z reads the description under the cursor, writes Far's
//! line, re-lists the panel, and walks the marked entries in turn.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{accept, open};
use crate::models::{App, FileName, Overlay, Panel};
use crate::status::Status;

fn prompt(app: &App) -> &crate::models::DescribePrompt {
    match &app.overlay {
        Overlay::Describe(prompt) => prompt,
        _ => panic!("expected the describe prompt to be open"),
    }
}

fn described(app: &App, name: &str) -> Option<String> {
    app.get_active_panel_ref()
        .entries
        .iter()
        .find(|entry| entry.name == name)
        .and_then(|entry| entry.description.clone())
}

mod description_files;
mod prompting;
