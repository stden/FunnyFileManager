//! Viewer, editor and modal operation regression tests.

use crate::scroll::ScrollTop;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

#[cfg(feature = "test-helpers")]
use crate::editor::{handle_edit_test_mode, TestEditorBehavior};

use crate::models::{App, Overlay, Panel, Viewer, Viewport};

use crossterm::event::KeyCode;

use std::fs::{self, File};

use std::io::{self, Write};

use tempfile::TempDir;

#[cfg(feature = "test-helpers")]
mod edit_files;
mod view_files;
mod viewer_keys;
