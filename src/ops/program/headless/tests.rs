//! Purpose: Private program launches preserve output, cwd, failures and refreshed panels.

use super::*;
use crate::{
    models::{Panel, TerminalMode},
    terminal::headless::HeadlessHost,
    WritableTestBackend,
};
use ratatui::Terminal;
use tempfile::TempDir;

fn fixture(name: &str, contents: &str) -> io::Result<(TempDir, App)> {
    let root = TempDir::new()?;
    std::fs::write(root.path().join(name), contents)?;
    let panel = Panel::new(root.path().to_owned())?;
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(
        app.left_panel.select_named(name),
        "select the fixture program"
    );
    app.live.terminal_mode = TerminalMode::Headless;
    Ok((root, app))
}

#[test]
fn private_program_keeps_exit_code_output_and_panel_directory() -> io::Result<()> {
    let (root, mut app) = fixture(
        "my program.sh",
        "printf tool-output; printf created >result.txt; exit 7\n",
    )?;
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24))?;
    crate::ops::program::launch(&mut terminal, &mut app)?;
    assert!(app.status_text().contains("exited with code 7"));
    assert_eq!(
        std::fs::read_to_string(root.path().join("result.txt"))?,
        "created"
    );
    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|entry| entry.name == "result.txt"));
    assert!(app
        .console
        .visible(100)
        .iter()
        .any(|line| line.text().contains("tool-output")));
    assert_eq!(
        host.snapshot().state,
        headless::ExternalState::Exited { code: 7 }
    );
    Ok(())
}

#[test]
fn removed_program_reports_the_literal_path_and_refreshes_the_panel() -> io::Result<()> {
    let (root, mut app) = fixture("missing-tool", "unused")?;
    let path = root.path().join("missing-tool");
    std::fs::remove_file(&path)?;
    let host = HeadlessHost::new();
    let _scope = host.enter();
    launch(&mut app)?;
    assert!(
        matches!(&app.status, Status::CannotRun { program, error } if program == &path && !error.is_empty())
    );
    assert!(!app
        .left_panel
        .entries
        .iter()
        .any(|entry| entry.name == "missing-tool"));
    Ok(())
}

#[test]
fn empty_selection_does_not_require_a_private_terminal() -> io::Result<()> {
    let root = TempDir::new()?;
    let panel = Panel::from_entries(root.path().to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    assert!(!headless::installed());
    launch(&mut app)?;
    assert_eq!(app.console.visible(10).len(), 0);
    Ok(())
}
