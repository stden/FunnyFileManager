//! Unit tests: deletes that partly fail, the drive dialog's keys, source
//! paths without marks, and transfers with the rename fallback.

use crate::models::FileName;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

use crate::models::{App, Overlay, Panel, PanelSide};

use crossterm::event::KeyCode;

use std::fs;

use std::io;

use tempfile::TempDir;

mod deletes;
mod drive_keys;
mod selection_and_errors;
mod transfers;
