//! Unit tests for [`super`], moved out of the module so their
//! assertion-failure arms are not measured as unreached production lines.

use super::*;
use std::fs;
use tempfile::TempDir;

mod mcp_status;
mod viewing;
