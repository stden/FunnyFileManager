//! Unit tests: the typed built-ins and `Command::parse`, a shell command's
//! output on the status bar, the paths an operation takes, and F7/F8 with
//! nothing to do.

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
