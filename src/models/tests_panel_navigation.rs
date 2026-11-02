//! Unit tests: panel navigation — the app's active and inactive panels, the
//! cursor through a listing, Enter on a row, and the selection.

use std::fs;

use super::*;

use std::fs::File;

use std::io::Write;

use tempfile::TempDir;

mod tests_app_panels;
mod tests_cursor;
mod tests_enter;
mod tests_selection;
