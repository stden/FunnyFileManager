//! Purpose: Implements built-in and external editor entrypoints for F4 workflows.
//!
use ratatui::backend::Backend;
use ratatui::Terminal;
use std::io::{self, Write};

use crate::models::App;
use crate::status::Status;

mod builtin;

pub(crate) use builtin::editor_for;
pub use builtin::handle_edit_builtin;

/// Fallback when `$EDITOR` is unset.
const DEFAULT_EDITOR: &str = "nano";

#[cfg(all(windows, not(any(test, coverage))))]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(all(windows, not(any(test, coverage))))]
fn configure_background_command(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(all(not(windows), not(any(test, coverage))))]
const fn configure_background_command(_command: &mut std::process::Command) {}

#[cfg(not(any(test, coverage)))]
fn command_succeeds_in_background(program: &str, args: &[&str]) -> bool {
    let mut command = std::process::Command::new(program);
    configure_background_command(&mut command);
    command
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success())
}

pub fn handle_edit<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    let mut terminal_ops = crate::terminal::CrosstermOps;
    handle_edit_with(terminal, app, &mut terminal_ops, |editor, args, path| {
        std::process::Command::new(editor)
            .args(args)
            .arg(path)
            .status()
    })
}

/// Split `$EDITOR` into program plus arguments, the way `EDITOR="code -w"` is
/// conventionally written.
///
/// A value that names an existing file wins over the split: an editor installed
/// under `C:\Program Files\...` or any path with a space is one program, not a
/// program plus arguments.
fn split_editor_command(env_editor: &str) -> (String, Vec<String>) {
    if std::path::Path::new(env_editor).is_file() {
        return (env_editor.to_string(), Vec::new());
    }
    let mut parts = env_editor.split_whitespace();
    let program = parts.next().unwrap_or(DEFAULT_EDITOR).to_string();
    (
        program,
        parts.map(std::string::ToString::to_string).collect(),
    )
}

/// Pick the editor to launch: VS Code when installed, else `$EDITOR`.
fn resolve_editor() -> (String, Vec<String>) {
    // Keep test builds deterministic and side-effect free by skipping the VS
    // Code probe there; the $EDITOR path stays shared so it is actually tested.
    #[cfg(not(any(test, coverage)))]
    if command_succeeds_in_background("code", &["--version"]) {
        return ("code".to_string(), vec!["-w".to_string(), "-n".to_string()]);
    }
    split_editor_command(&std::env::var("EDITOR").unwrap_or_else(|_| DEFAULT_EDITOR.to_string()))
}

/// Shared body of [`handle_edit`], with terminal and process operations injected.
fn handle_edit_with<B, F, O>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    terminal_ops: &mut O,
    run_editor: F,
) -> io::Result<()>
where
    B: Backend + Write,
    O: crate::terminal::TerminalOps<B>,
    F: FnOnce(&str, &[String], &std::path::Path) -> io::Result<std::process::ExitStatus>,
{
    let Some(selected_entry) = app.get_active_panel().get_selected_entry() else {
        return Ok(());
    };

    if selected_entry.kind.is_dir() {
        app.set_status(Status::EditDirUnsupported);
        return Ok(());
    }

    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    let (editor, args) = resolve_editor();
    // Remembered on launch, as Far does: the file was opened, whatever the
    // editor then made of it.
    app.view_history.touch(&selected);

    let suspension = crate::terminal::TerminalSuspension::suspend(terminal, terminal_ops)?;
    let status = run_editor(&editor, &args, &selected);
    let resume_result = suspension.resume();

    app.set_status(editor_outcome(status, &selected));

    let refresh_result = crate::utils::refresh_panels(app);
    crate::terminal::merge_run_and_cleanup(refresh_result, resume_result)
}

fn editor_outcome(
    status: io::Result<std::process::ExitStatus>,
    selected: &std::path::Path,
) -> Status {
    match status {
        Ok(code) if code.success() => Status::Edited(selected.display().to_string()),
        Ok(code) => Status::EditorExited {
            status: code.to_string(),
        },
        Err(error) => Status::LaunchEditorFailed {
            error: error.to_string(),
        },
    }
}

#[cfg(any(test, coverage))]
#[cfg_attr(coverage, allow(dead_code))]
pub(crate) fn handle_edit_impl<B, F>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    run_editor: F,
) -> io::Result<()>
where
    B: Backend + Write,
    F: FnOnce(&str, &[String], &std::path::Path) -> io::Result<std::process::ExitStatus>,
{
    let mut terminal_ops = crate::terminal::NoopTerminalOps;
    handle_edit_with(terminal, app, &mut terminal_ops, run_editor)
}

#[cfg(any(test, coverage, feature = "test-helpers"))]
#[derive(Clone, Copy, Debug)]
pub enum TestEditorBehavior {
    Success,
    FailLaunch,
    ExitError,
}

#[cfg(any(test, coverage, feature = "test-helpers"))]
pub fn handle_edit_test_mode(app: &mut App, behavior: TestEditorBehavior) -> io::Result<()> {
    let Some(selected_entry) = app.get_active_panel().get_selected_entry() else {
        return Ok(());
    };

    if selected_entry.kind.is_dir() {
        app.set_status(Status::EditDirUnsupported);
        return Ok(());
    }

    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    match behavior {
        TestEditorBehavior::FailLaunch => {
            app.set_status(Status::LaunchEditorFailed {
                error: "No such file or directory (os error 2)".to_string(),
            });
        }
        TestEditorBehavior::ExitError => {
            app.set_status(Status::EditorExited {
                status: "exit status: 1".to_string(),
            });
        }
        TestEditorBehavior::Success => {
            app.set_status(Status::Edited(selected.display().to_string()));
        }
    }

    app.left_panel.load_entries()?;
    app.left_panel.ensure_selection();
    app.right_panel.load_entries()?;
    app.right_panel.ensure_selection();
    Ok(())
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_external_launch;
#[cfg(test)]
mod tests_guards;
// FIFOs and device nodes are the inputs these tests need; neither exists on
// Windows, so the whole module is Unix-only.
#[cfg(all(test, unix))]
mod tests_special_files;

#[cfg(test)]
mod legacy_unit_tests;
