//! Application event-loop and input interaction tests.

use super::*;
use crate::models::{App, Lifecycle, Overlay};
#[cfg(feature = "test-helpers")]
use crate::test_support::select_entry;
use crate::test_support::{buffer_text, key_event, key_event_with_mods};
use crate::{models, WritableTestBackend};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

mod command_line_help;
mod event_loop_misc;
mod event_sequences;
mod fkey_dialogs;
mod idle_render;
mod make_folder;
mod marking_and_panels;
mod quick_search_debounce;
mod quick_search_nav;
mod replay_resize;
mod restore_selection;
mod run_lifecycle;
mod selection_groups;
