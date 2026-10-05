//! Dispatcher tests retain private access without widening runtime APIs.

use super::*;
use crate::models::{App, Overlay, ViewMode};
use crate::test_support::{buffer_text, key_event, key_event_with_mods};
use crate::{menu, models, ops, status, WritableTestBackend};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::path::{Path, PathBuf};

use crate::application::run_app_with_event_reader;
use crate::ui::ui;

mod action_dispatch;
mod dispatch_errors;
mod event_loop_keys;
mod event_loop_toggles;
mod far_menu_families;
mod netbox_panels;
mod panel_views;
