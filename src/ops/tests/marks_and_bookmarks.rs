//! Unit tests: typed bookmark, mark, sort, help and quit commands, F8's
//! confirmation, and the viewer on a one-line file.

use crate::models::FileName;
use crate::scroll::ScrollTop;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

use crate::models::{App, BookmarkSlot, Overlay, Panel, SortMode, Viewer};

use crossterm::event::KeyCode;

use std::fs::{self, File};

use std::io;

use tempfile::TempDir;

mod bookmark_commands;
mod confirm_dialog;
mod help_quit_and_aliases;
mod marks_and_sorting;
mod viewer;
