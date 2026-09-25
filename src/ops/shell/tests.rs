//! Purpose: Unit tests for shell command execution.
//! Kept beside the canonical `ops::shell` owner.
use super::*;
use tempfile::TempDir;

mod bounds;
mod running;
#[cfg(unix)]
mod stopping;
