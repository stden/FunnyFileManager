//! Unit tests: panel sorting (split out to keep files under 500 lines).

use std::fs;

use super::tests::*;

use super::*;

use std::fs::File;

use std::io::Write;

use tempfile::TempDir;

mod tests_cursor_edges;
mod tests_extremes;
mod tests_marks;
mod tests_sort_modes;
