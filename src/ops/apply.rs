//! Purpose: Apply command (`Ctrl+G`): open the prompt, then run its template once per selected entry.
//!
//! Every run goes through the same shell as a `:` command and is echoed into
//! the console transcript with its substituted line, so `Ctrl+O` afterwards
//! shows exactly what was executed for which file — the part of Far's
//! behaviour that makes a bulk command auditable rather than magical.

use std::io;
use std::path::Path;

use crate::models::apply::{expand, Context, Symbol, Token, Tokens};
use crate::models::{App, ApplyPrompt, Overlay};
use crate::status::Status;
use crate::utils::refresh_panels;

use super::shell::{run_shell_command, ShellError};

mod context;
mod lists;

#[derive(Clone, Copy, PartialEq, Eq)]
enum StepOutcome {
    Succeeded,
    Failed,
    Cancelled,
}

/// Ctrl+G: open the Apply command dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::ApplyCommand(ApplyPrompt::default()));
}

/// Run `template` once for each marked entry — or the one under the cursor —
/// in the active panel's directory, then re-read the panels.
pub fn run(app: &mut App, template: &str) {
    run_with_list_dir(app, template, &std::env::temp_dir());
}

/// [`run`], with the `!@` list written under `list_dir` — the temp directory
/// in production, whatever the test needs to make the write fail otherwise.
fn run_with_list_dir(app: &mut App, template: &str, list_dir: &Path) {
    let names = context::selected_names(app.get_active_panel());
    if names.is_empty() {
        app.set_status(Status::CommandOutput(
            "Apply command: nothing selected".to_owned(),
        ));
        return;
    }
    let cwd = app.get_active_panel().path.clone();
    app.get_active_panel().remember_marks();
    let status = match run_batch(app, template, &names, &cwd, list_dir) {
        Ok(failed) => summary(names.len(), failed),
        Err(error) => {
            Status::CommandOutput(format!("Apply command: cannot write the !@ list: {error}"))
        }
    };
    app.set_status(status);
    // The commands may have created or removed files; a refresh failure is
    // not theirs, so it is deliberately not reported over their summary.
    drop(refresh_panels(app));
}

fn run_batch(
    app: &mut App,
    template: &str,
    names: &[String],
    cwd: &Path,
    list_dir: &Path,
) -> io::Result<usize> {
    let mut context = context::capture(app, names.first().map_or("", String::as_str));
    let mut failed = 0;
    for (idx, name) in names.iter().enumerate() {
        context::select(&mut context, app, name);
        let outcome = run_entry_step(app, template, &context, cwd, list_dir)?;
        if outcome == StepOutcome::Cancelled {
            failed += names.len() - idx;
            break;
        }
        context
            .panel_mut(context.active)
            .selected
            .retain(|entry| entry.long != *name);
        if outcome == StepOutcome::Succeeded {
            app.get_active_panel().marked.remove(name.as_str());
        } else {
            failed += 1;
        }
    }
    Ok(failed)
}

#[cfg(test)]
fn run_entry(
    app: &mut App,
    template: &str,
    context: &Context,
    cwd: &Path,
    list_dir: &Path,
) -> io::Result<bool> {
    run_entry_step(app, template, context, cwd, list_dir)
        .map(|outcome| outcome == StepOutcome::Succeeded)
}

fn run_entry_step(
    app: &mut App,
    template: &str,
    context: &Context,
    cwd: &Path,
    list_dir: &Path,
) -> io::Result<StepOutcome> {
    let mut lists = lists::Lists::new(list_dir);
    let command = expand(template, context, |panel, options| {
        lists.write(panel, options)
    })?;
    let outcome = if command.is_empty() {
        StepOutcome::Succeeded
    } else {
        run_one(app, cwd, &command)
    };
    if Tokens::new(template).any(|token| token == Token::Symbol(Symbol::PreserveName)) {
        let entry = &context.panel(context.active).entry;
        if let Err(error) = context::restore_long_name(cwd, entry) {
            app.console.record_error(&format!(
                "cannot restore long name '{}': {error}",
                entry.long
            ));
            return Ok(StepOutcome::Failed);
        }
    }
    Ok(outcome)
}

/// One run, echoed and recorded like a `:` command.
fn run_one(app: &mut App, cwd: &Path, command: &str) -> StepOutcome {
    app.console.record_command(cwd, command);
    match run_shell_command(command, cwd) {
        Ok(output) => {
            app.console
                .record_output(&String::from_utf8_lossy(&output.stdout));
            app.console
                .record_error(&String::from_utf8_lossy(&output.stderr));
            if output.status.success() {
                StepOutcome::Succeeded
            } else {
                StepOutcome::Failed
            }
        }
        Err(error) => {
            let cancelled = matches!(error, ShellError::Cancelled);
            app.console.record_error(&error.report(command));
            if cancelled {
                StepOutcome::Cancelled
            } else {
                StepOutcome::Failed
            }
        }
    }
}

fn summary(total: usize, failed: usize) -> Status {
    if failed == 0 {
        return Status::CommandOutput(format!("Apply command: {total} run"));
    }
    Status::CommandOutput(format!("Apply command: {failed} of {total} failed"))
}

#[cfg(test)]
mod tests;
