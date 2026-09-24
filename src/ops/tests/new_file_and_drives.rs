//! Unit tests: new file and drives (split out to keep files under 500 lines).

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
