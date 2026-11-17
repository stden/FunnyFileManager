//! Unit tests: copy and move edge cases, the keys each overlay receives, and
//! the viewer overlay's edges.

use crate::models::FileName;
use crate::models::Lifecycle;

use super::support::*;
use crate::ops::*;
use crate::popup::process_popup_input;

use crate::models::{App, CommandLine, Overlay, PanelSide, Viewer};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use std::fs::{self, File};

use std::io::{self, Write};

#[cfg(not(coverage))]
use std::path::{Path, PathBuf};

use tempfile::TempDir;

mod popup_triggers;
mod transfer_edges;
mod viewer_edges;
