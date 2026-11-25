//! Unit tests: the command-line prompt — its keys, the typed built-ins, Tab
//! completion's menu, history and its ghost suggestion — and the file
//! operations reached from it.

use crate::lineedit::LineEditor;
use crate::models::FileName;
use crate::models::Lifecycle;
use crate::models::Recall;
use crate::status::Status;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

use crate::models::{
    App, BookmarkSlot, Candidate, CandidateKind, CommandLine, Menu, Overlay, Panel, SortMode,
};

use crossterm::event::{KeyCode, KeyModifiers};

use std::fs::{self, File};

use std::io::{self, Write};

use std::path::Path;

use tempfile::TempDir;

/// The open dropdown's candidate texts, for readable list assertions.
fn menu_texts(app: &App) -> Vec<String> {
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should be open");
    };
    state
        .menu
        .as_ref()
        .expect("menu should be open")
        .candidates()
        .iter()
        .map(|c| c.text.clone())
        .collect()
}

/// The command line's current buffer text.
fn buffer_of(app: &App) -> String {
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should be open");
    };
    state.text().to_owned()
}

mod completion_menu;
mod execute_branches;
mod file_operations;
mod history_ghost;
mod menu_refresh;
mod prompt_keys;
