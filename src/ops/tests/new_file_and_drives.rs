//! Unit tests: Enter in the drive dialog and typing at the command line, new
//! files and folders, shell output on the status bar, and F3/F8 corner cases.

use crate::models::FileName;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

use crate::models::{App, BookmarkSlot, Overlay, Panel};

use crossterm::event::KeyCode;

use std::fs::{self, File};

use std::io;

use tempfile::TempDir;

mod drives_and_command_line;
mod new_file;
mod shell_output;
mod view_and_sources;
