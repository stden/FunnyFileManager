//! Unit tests: panel navigation (split out to keep files under 500 lines).

use std::fs;

use super::*;

use std::fs::File;

use std::io::Write;

use tempfile::TempDir;

mod tests_app_panels;
mod tests_cursor;
mod tests_enter;
mod tests_selection;
