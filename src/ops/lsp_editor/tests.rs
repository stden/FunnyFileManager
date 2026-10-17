//! Purpose: Pin how the dialog opens on the settings file, what F2 writes, and
//! that every refused or failed save leaves the file and the draft alone.

use std::{fs, io};

use super::save::save_with;
use super::*;
use crate::config::Enablement;
use crate::lsp::NotInstalled;
use crate::models::lsp_editor::Mode;
use crate::models::settings::Settings;

use crate::test_support::lsp_machine::{editor, editor_mut, LspMachine as Machine};

/// `text` as YAML, to compare with what the file holds.
fn yaml(text: &str) -> serde_norway::Value {
    serde_norway::from_str(text).expect("YAML")
}

#[path = "tests_edit.rs"]
mod edit;

mod opening;
mod saving;
