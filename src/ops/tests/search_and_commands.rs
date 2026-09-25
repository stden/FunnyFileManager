//! Unit tests: F3 and command-line error paths, invalid typed commands, and
//! the viewer's limits.

use crate::scroll::ScrollTop;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

use crate::models::{App, CommandLine, Overlay, Viewer};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::fs::{self, File};

use std::io::{self, Write};

use tempfile::TempDir;

mod error_paths;
mod invalid_commands;
mod viewer_limits;
