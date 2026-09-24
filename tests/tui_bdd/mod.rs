//! Gherkin steps only interact with the running UI and isolated fixture files.

use std::{
    fs,
    path::{Path, PathBuf},
};

use cucumber::{given, then, when};
use ratatui::layout::Size;
use tempfile::TempDir;

mod file_steps;
mod input_steps;
mod lsp;
mod restart;
mod screen_steps;
mod session;
mod settings;
#[cfg(feature = "keepass")]
mod vault;
mod workspace_steps;

use session::Session;

#[derive(cucumber::World, Debug, Default)]
pub struct TuiWorld {
    pub name: String,
    workspace: Option<TempDir>,
    session: Option<Session>,
    /// The machine the MCP installer sees, when a scenario brings one.
    mcp: Option<funny_file_manager::test_support::FakeMachine>,
}

impl TuiWorld {
    fn path(&self, side: &str, file: &str) -> PathBuf {
        assert!(matches!(side, "left" | "right"));
        self.root().join(side).join(file)
    }

    #[expect(
        clippy::expect_used,
        reason = "the workspace step runs first; a scenario that skips it must fail here"
    )]
    fn root(&self) -> &Path {
        self.workspace.as_ref().expect("workspace").path()
    }

    #[expect(
        clippy::expect_used,
        reason = "the open step runs first; a scenario that skips it must fail here"
    )]
    fn ui(&mut self) -> &mut Session {
        self.session.as_mut().expect("TUI is open")
    }

    #[expect(
        clippy::expect_used,
        reason = "a scenario whose transcript cannot be saved must fail, not lose its evidence"
    )]
    pub fn finish(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.stop();
            session.save_trace(&self.name).expect("save TUI transcript");
        }
    }
}
