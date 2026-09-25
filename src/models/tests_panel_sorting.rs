//! Unit tests: the sort modes, marks, the cursor at the edges, and extreme
//! names.

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
