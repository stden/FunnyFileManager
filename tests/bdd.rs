//! Purpose: Maps English Cucumber scenarios to `FunnyFileManager` test steps.
//!
#![expect(
    clippy::future_not_send,
    reason = "Cucumber generates a single-threaded world constructor owning the UI clipboard adapter"
)]
#![expect(
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    reason = "Cucumber dictates every step's signature: captured args arrive owned and the world as `&mut`"
)]

use bdd_runner::{Context as _, StepResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use cucumber::{given, then, when, World as _};
use funny_file_manager::description;
use funny_file_manager::editor::{handle_edit_test_mode, TestEditorBehavior};
use funny_file_manager::handlers::{
    handle_copy, handle_delete, handle_delete_confirm, handle_mkdir, handle_move, handle_view,
    process_popup_input,
};
use funny_file_manager::models::BookmarkSlot;
use funny_file_manager::models::FileName;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::Lifecycle;
use funny_file_manager::models::ViewMode;
use funny_file_manager::models::{
    App, NamePrompt, Overlay, Panel, PanelSide, PanelsShown, SortMode,
};
use funny_file_manager::ops::panels;
use funny_file_manager::s3::{list_aws_profiles, VirtualPath};
use funny_file_manager::status::Status;
use funny_file_manager::ui::ui;
use funny_file_manager::utils::{copy_dir_all, unique_destination};
use funny_file_manager::{run_app_with_event_reader, WritableTestBackend};
use ratatui::Terminal;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

#[path = "bdd/app_state.rs"]
mod app_state;
#[path = "bdd/command_line.rs"]
mod command_line;
#[path = "bdd/descriptions.rs"]
mod descriptions;
#[path = "bdd/editor.rs"]
mod editor;
#[path = "bdd/fs_fixtures.rs"]
mod fs_fixtures;
#[path = "bdd/marks_and_bookmarks.rs"]
mod marks_and_bookmarks;
#[path = "bdd/navigation.rs"]
mod navigation;
#[path = "bdd/navigation_checks.rs"]
mod navigation_checks;
#[path = "bdd/new_items.rs"]
mod new_items;
#[path = "bdd/screen.rs"]
mod screen;
#[path = "bdd/screen_checks.rs"]
mod screen_checks;
#[path = "bdd/sorting.rs"]
mod sorting;
#[path = "bdd/transfer.rs"]
mod transfer;
#[path = "bdd/transfer_checks.rs"]
mod transfer_checks;
#[path = "bdd/utilities.rs"]
mod utilities;
#[path = "bdd/world.rs"]
mod world;

// ─── World ──────────────────────────────────────────────────────

#[derive(cucumber::World, Default)]
struct FfmWorld {
    temp_dir: Option<TempDir>,
    src_dir: Option<TempDir>,
    dst_dir: Option<TempDir>,
    app: Option<App>,
    panel: Option<Panel>,
    // For VirtualPath scenarios
    parsed_path: Option<VirtualPath>,
    // For utility scenarios
    formatted_size: Option<String>,
    unique_name: Option<String>,
    aws_profiles: Vec<String>,
    original_left_path: Option<PathBuf>,
    // For F9 Commands > Install MCP server scenarios
    mcp: Option<mcp_install::McpScenario>,
}

impl std::fmt::Debug for FfmWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FfmWorld")
            .field("temp_dir", &self.temp_dir.as_ref().map(TempDir::path))
            .field("src_dir", &self.src_dir.as_ref().map(TempDir::path))
            .field("dst_dir", &self.dst_dir.as_ref().map(TempDir::path))
            .field("has_app", &self.app.is_some())
            .field("has_panel", &self.panel.is_some())
            .field("parsed_path", &self.parsed_path)
            .field("formatted_size", &self.formatted_size)
            .field("unique_name", &self.unique_name)
            .field("aws_profiles", &self.aws_profiles)
            .field("original_left_path", &self.original_left_path)
            .field("mcp", &self.mcp)
            .finish()
    }
}

fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

fn push_text(events: &mut Vec<Event>, text: &str) {
    events.extend(text.chars().map(|ch| key_event(KeyCode::Char(ch))));
}

fn push_quit(events: &mut Vec<Event>) {
    events.push(Event::Key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    )));
}

#[path = "support/bdd_runner.rs"]
mod bdd_runner;
#[path = "bdd/mcp_install.rs"]
mod mcp_install;

fn main() -> io::Result<()> {
    // Cucumber prints the world only when a step fails, so a green run never
    // called this impl. Formatting the empty world executes every field.
    let rendered = format!("{:?}", FfmWorld::default());
    assert!(rendered.starts_with("FfmWorld"), "{rendered}");
    bdd_runner::cover_error_paths();
    bdd_runner::run(|| futures::executor::block_on(Box::pin(FfmWorld::run("tests/features"))))
}
