//! Unit tests: shell commands (split out to keep files under 500 lines).

use crate::models::FileName;

use crate::ops::*;

use crate::command::Command;

use crate::models::{App, BookmarkSlot, Overlay, Panel, SortMode};

use std::fs;

use std::io;

use tempfile::TempDir;

mod builtins;
mod command_parser;
mod delete_and_mkdir;
mod shell_output;
mod source_paths;
