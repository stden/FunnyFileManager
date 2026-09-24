//! Unit tests for `ui`, one child module per behaviour under `ui/tests/`.
//!
//! The imports below are what each child's own `use super::*;` reaches the
//! parent's items through: `ui`'s, and the few `ui.rs` itself no longer uses.

use super::fkey_bar::{render_key_bar, FKEY_LABELS};
use super::panel::{panel_footer, render_panel};
use super::*;
use crate::models::PanelSide;

mod completion_menu;
mod console_keys;
mod console_layer;
mod console_prompt;
mod console_screens;
mod editor_exit;
mod editor_exit_layout;
mod editor_title;
mod ghost_text;
mod info_row_failures;
mod info_row_fit;
mod info_row_messages;
mod netbox_dialogs;
mod netbox_layout;
mod netbox_layout_dialogs;
mod netbox_remote;
mod netbox_sessions;
mod overlay_confirm;
mod overlay_controls;
mod overlay_drives;
mod overlay_full_screen;
mod overlay_mask;
mod overlay_name_prompt;
mod overlay_status;
mod panel_geometry;
mod panel_rendering;
mod render_gaps_dialogs;
mod render_gaps_editor;
mod render_widths;
mod status_line;
mod ui_render;
mod widths_cells;
mod widths_command_line;
mod widths_fields;
mod widths_key_bar;
mod widths_panels;
mod widths_scrolling;
