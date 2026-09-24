//! Unit tests: copy move (split out to keep files under 500 lines).

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
