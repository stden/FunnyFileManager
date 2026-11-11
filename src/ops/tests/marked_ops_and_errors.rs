//! Unit tests: F8 edge cases, the drive dialog's cursor, transfers of marked
//! files, and F3's error paths.

use crate::models::EntryKind;
use crate::models::FileName;
use crate::models::FileSize;
use crate::models::Modified;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

use crate::models::{App, Overlay, Panel, PanelSide};

use crossterm::event::KeyCode;

use std::fs;

use std::io;

use tempfile::TempDir;

mod delete_edges;
mod drive_wrapping;
mod marked_transfers;
mod view_errors;
