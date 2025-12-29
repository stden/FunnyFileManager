#[cfg(all(not(test), not(tarpaulin)))]
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::io::{self, Write};

#[cfg(all(not(test), not(tarpaulin)))]
use std::process::Command;

use crate::models::App;

#[cfg(any(test, tarpaulin))]
pub fn handle_edit<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let _ = terminal;
    handle_edit_test_mode(app, TestEditorBehavior::Success)
}

#[cfg(all(not(test), not(tarpaulin)))]
pub fn handle_edit<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    handle_edit_impl(terminal, app, |editor, path| {
        Command::new(editor).arg(path).status()
    })
}

pub(crate) fn handle_edit_impl<B, F>(
    #[allow(unused_variables)] terminal: &mut Terminal<B>,
    app: &mut App,
    run_editor: F,
) -> io::Result<()>
where
    B: Backend + Write,
    F: FnOnce(&str, &std::path::Path) -> io::Result<std::process::ExitStatus>,
{
    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    if selected.is_dir() {
        app.status_message = "Editing directories is not supported".to_string();
        return Ok(());
    }

    // Skip terminal manipulation in test mode.
    #[cfg(all(not(test), not(tarpaulin)))]
    {
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let status = run_editor(&editor, &selected);

    #[cfg(all(not(test), not(tarpaulin)))]
    {
        enable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        terminal.clear()?;
    }

    match status {
        Ok(code) => {
            if code.success() {
                app.status_message = format!("Edited {}", selected.display());
            } else {
                app.status_message = format!("Editor exited with status {}", code);
            }
        }
        Err(err) => {
            app.status_message = format!("Failed to launch editor: {}", err);
        }
    }

    app.left_panel.load_entries()?;
    app.left_panel.ensure_selection();
    app.right_panel.load_entries()?;
    app.right_panel.ensure_selection();
    Ok(())
}

#[cfg(any(test, tarpaulin))]
#[derive(Clone, Copy, Debug)]
pub(crate) enum TestEditorBehavior {
    Success,
    FailLaunch,
    ExitError,
}

#[cfg(any(test, tarpaulin))]
pub(crate) fn handle_edit_test_mode(app: &mut App, behavior: TestEditorBehavior) -> io::Result<()> {
    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    if selected.is_dir() {
        app.status_message = "Editing directories is not supported".to_string();
        return Ok(());
    }

    match behavior {
        TestEditorBehavior::FailLaunch => {
            app.status_message =
                "Failed to launch editor: No such file or directory (os error 2)".to_string();
        }
        TestEditorBehavior::ExitError => {
            app.status_message = "Editor exited with status exit status: 1".to_string();
        }
        TestEditorBehavior::Success => {
            app.status_message = format!("Edited {}", selected.display());
        }
    }

    app.left_panel.load_entries()?;
    app.left_panel.ensure_selection();
    app.right_panel.load_entries()?;
    app.right_panel.ensure_selection();
    Ok(())
}
