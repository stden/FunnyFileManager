//! Unit tests: F5 and F6 — copying, moving across devices and as a rename —
//! and F7/F8 corner cases.

use super::support::*;
use crate::ops::*;

use crate::models::{App, Panel};

use std::fs::{self, File};

use std::io::{self, Write};

use tempfile::TempDir;

mod copying;
mod mkdir_and_delete;
mod move_fallbacks;
mod move_renames;
