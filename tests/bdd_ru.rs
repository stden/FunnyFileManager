//! Purpose: Maps Russian Cucumber scenarios to `FunnyFileManager` test steps.
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
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use cucumber::{given, then, when, World as _};
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
use funny_file_manager::models::{App, Overlay, Panel, PanelSide, SortMode};
use funny_file_manager::s3::VirtualPath;
use funny_file_manager::status::Status;
use funny_file_manager::utils::{copy_dir_all, unique_destination};
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tempfile::TempDir;

#[path = "bdd_ru/app_state.rs"]
mod app_state;
#[path = "bdd_ru/command_line.rs"]
mod command_line;
#[path = "bdd_ru/editor.rs"]
mod editor;
#[path = "bdd_ru/fs_fixtures.rs"]
mod fs_fixtures;
#[path = "bdd_ru/navigation.rs"]
mod navigation;
#[path = "bdd_ru/navigation_checks.rs"]
mod navigation_checks;
#[path = "bdd_ru/new_items.rs"]
mod new_items;
#[path = "bdd_ru/overlay_checks.rs"]
mod overlay_checks;
#[path = "bdd_ru/overlays.rs"]
mod overlays;
#[path = "bdd_ru/sorting_and_marks.rs"]
mod sorting_and_marks;
#[path = "bdd_ru/transfer.rs"]
mod transfer;
#[path = "bdd_ru/utilities.rs"]
mod utilities;

// ─── World ──────────────────────────────────────────────────────

#[derive(cucumber::World, Default)]
struct FfmWorld {
    temp_dir: Option<TempDir>,
    src_dir: Option<TempDir>,
    dst_dir: Option<TempDir>,
    app: Option<App>,
    panel: Option<Panel>,
    parsed_path: Option<VirtualPath>,
    formatted_size: Option<String>,
    unique_name: Option<String>,
    original_left_path: Option<PathBuf>,
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
            .field("original_left_path", &self.original_left_path)
            .finish()
    }
}

impl FfmWorld {
    fn temp_path(&self) -> StepResult<PathBuf> {
        Ok(self
            .temp_dir
            .as_ref()
            .context("нет временного каталога")?
            .path()
            .to_path_buf())
    }
    fn src_path(&self) -> StepResult<PathBuf> {
        Ok(self
            .src_dir
            .as_ref()
            .context("нет исходного каталога")?
            .path()
            .to_path_buf())
    }
    fn dst_path(&self) -> StepResult<PathBuf> {
        Ok(self
            .dst_dir
            .as_ref()
            .context("нет каталога назначения")?
            .path()
            .to_path_buf())
    }
    fn app(&mut self) -> StepResult<&mut App> {
        self.app.as_mut().context("нет приложения")
    }
    fn panel(&mut self) -> StepResult<&mut Panel> {
        self.panel.as_mut().context("нет панели")
    }
}

#[path = "support/bdd_runner.rs"]
mod bdd_runner;

fn main() -> std::io::Result<()> {
    bdd_runner::run(|| futures::executor::block_on(Box::pin(FfmWorld::run("tests/features_ru"))))
}
