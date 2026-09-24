//! Purpose: Exercises editor workflows from the public integration-test boundary.
//!

use funny_file_manager::models::EntryKind;
use funny_file_manager::models::FileSize;
use funny_file_manager::models::Modified;
use funny_file_manager::models::{App, DirEntry, Panel, PanelSide};
use std::fs::File;
use std::io;
use tempfile::TempDir;

#[cfg(feature = "test-helpers")]
use funny_file_manager::editor::{handle_edit_test_mode, TestEditorBehavior};

#[path = "editor_test/outcomes.rs"]
mod outcomes;
#[path = "editor_test/targets.rs"]
mod targets;

fn create_test_app(temp_dir: &TempDir) -> io::Result<App> {
    let mut app = App::with_panels(
        Panel::new(temp_dir.path().to_path_buf())?,
        Panel::new(temp_dir.path().to_path_buf())?,
    );
    app.status = funny_file_manager::status::Status::Cleared;
    Ok(app)
}

// The library dependency retains the real probe, which cfg(test) unit tests skip.
#[cfg(all(unix, not(coverage), feature = "test-helpers"))]
#[path = "support/editor_probe.rs"]
mod editor_probe;

#[cfg(all(unix, not(coverage), feature = "test-helpers"))]
#[test]
fn handle_edit_probes_for_the_code_cli_and_uses_it_when_present() -> io::Result<()> {
    editor_probe::verify()
}
