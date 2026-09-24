//! Unit tests: delete confirm shell (split out to keep files under 500 lines).

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

use crate::models::{App, Overlay, Panel, PanelSide};

use crossterm::event::KeyCode;

use std::fs::{self, File};

use std::io;

use std::path::PathBuf;

use tempfile::TempDir;

mod confirm_dialog;
mod drive_wrapping;
mod new_file_and_view;
mod shell_failures;
