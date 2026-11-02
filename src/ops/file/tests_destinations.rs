//! Unit tests: where an F5/F6 transfer lands, and what the status line says
//! about it afterwards.

use super::tests_transfer_policies::{app_with, mark, settings};
use super::*;
use crate::copy::{CopyPrompt, ExistingFiles, Purpose};

mod tests_onto_itself;
mod tests_typed_destinations;
