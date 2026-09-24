//! Unit tests: drive selection (split out to keep files under 500 lines).

use super::support::*;
use crate::popup::process_popup_input;

use crate::models::{App, Overlay, Panel, PanelSide};

use crossterm::event::{KeyCode, KeyModifiers};

use std::fs::{self, File};

use std::io;

use std::path::PathBuf;

use tempfile::TempDir;

mod delete_confirm_keys;
mod drive_keys;
mod new_file_keys;
