//! Unit tests: the archive runs behind the three dialogs — the command each
//! format builds, the transcript line that keeps a strange name one word, the
//! archives `tar` really writes, and the members an extraction refuses.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

#[cfg(unix)]
use super::command_line;
use super::members::{member_type, validate_listing};
use super::{
    create_archive, extract_archive, list_archive, offered_formats, on_path, test_archive,
    tests_extract, ArchiveRequest,
};
use crate::models::ArchiveFormat;

fn names(sources: &[&str]) -> Vec<String> {
    sources.iter().map(|name| (*name).to_owned()).collect()
}

mod adding;
mod reading;
