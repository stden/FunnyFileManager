//! Program arguments and terminal handoff use the same production launch path.
use super::*;
use crate::{models::Panel, terminal::NoopTerminalOps, WritableTestBackend};
use tempfile::TempDir;

fn fixture() -> (TempDir, App, Terminal<WritableTestBackend>) {
    let root = TempDir::new().expect("program directory");
    std::fs::write(root.path().join("my program.exe"), b"fixture").expect("file");
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(root.path().to_path_buf()).expect("panel");
    app.right_panel = Panel::new(root.path().to_path_buf()).expect("panel");
    let index = app
        .left_panel
        .entries
        .iter()
        .position(|entry| entry.name == "my program.exe");
    app.left_panel.select(index);
    let terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    (root, app, terminal)
}

fn transcript(app: &App) -> Vec<String> {
    app.console
        .visible(usize::MAX)
        .into_iter()
        .map(|line| line.text().to_owned())
        .collect()
}

/// Remembers every pause prompt; otherwise does nothing.
#[derive(Default)]
struct PauseOps {
    prompts: Vec<String>,
}

impl TerminalOps<WritableTestBackend> for PauseOps {
    fn enable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn disable_raw(&mut self) -> io::Result<()> {
        Ok(())
    }
    fn enter_alternate(&mut self, _: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
    fn leave_alternate(&mut self, _: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
    fn enable_mouse(&mut self, _: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
    fn disable_mouse(&mut self, _: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
    fn clear(&mut self, _: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
    fn show_cursor(&mut self, _: &mut Terminal<WritableTestBackend>) -> io::Result<()> {
        Ok(())
    }
    fn wait_for_key(
        &mut self,
        _: &mut Terminal<WritableTestBackend>,
        prompt: &str,
    ) -> io::Result<()> {
        self.prompts.push(prompt.to_owned());
        Ok(())
    }
}

mod launching;
mod pause_after_run;
