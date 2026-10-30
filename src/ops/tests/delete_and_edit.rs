//! Unit tests: F8 of files and folders, keys the viewer and help ignore, and
//! F4 through the editor seam.

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

#[cfg(feature = "test-helpers")]
use crate::editor::{handle_edit_impl, handle_edit_test_mode, TestEditorBehavior};

use crate::models::{App, Overlay, Panel, Viewer};

#[cfg(feature = "test-helpers")]
use crate::WritableTestBackend;

use crossterm::event::KeyCode;

use std::fs::{self, File};

use std::io;

use tempfile::TempDir;

mod deletes_and_keys;
#[cfg(feature = "test-helpers")]
mod editing;
