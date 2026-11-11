//! Purpose: Show a temp-only preview and an explicit default-No deletion question.
use super::dialog::{self, Button, Ground, Row};
use crate::models::temp_cleanup::{TempChoice, TempCleanupState, TempOutcome, TempPlan};
use crate::models::{Confirm, TempCleanupDialog};
use ratatui::Frame;

const TITLE: &str = "Clean temporary files";

pub(super) fn render(
    frame: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &TempCleanupDialog,
) {
    render_state(frame, tracker, &dialog.state);
}

fn render_state(
    frame: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    state: &TempCleanupState,
) {
    match state {
        TempCleanupState::QueuedScan(_) | TempCleanupState::Scanning => {
            message(frame, tracker, "Scanning temporary files...", "&Cancel");
        }
        TempCleanupState::Preview { plan, focus } => preview(frame, tracker, plan, *focus),
        TempCleanupState::Confirm { plan, answer } => confirm(frame, tracker, plan, *answer),
        TempCleanupState::QueuedDelete(_) | TempCleanupState::Deleting => {
            message(
                frame,
                tracker,
                "Deleting confirmed temporary files...",
                "&Close",
            );
        }
        TempCleanupState::Finished(outcome) => finished(frame, tracker, outcome),
        TempCleanupState::Failed(error) => message(frame, tracker, error.message(), "&Close"),
    }
}

fn preview(
    frame: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    plan: &TempPlan,
    focus: TempChoice,
) {
    let lines = details(plan);
    let rows: Vec<_> = lines.iter().map(|line| Row::Plain(line)).collect();
    let buttons = [
        Button::new("&Clean up")
            .enabled(!plan.files.is_empty())
            .focused(focus == TempChoice::Clean),
        Button::new("&Cancel")
            .default_button()
            .focused(focus == TempChoice::Cancel),
    ];
    dialog::render_tracked(frame, tracker, TITLE, Ground::Prompt, &rows, &buttons);
}

fn confirm(
    frame: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    plan: &TempPlan,
    answer: Confirm,
) {
    let mut lines = details(plan);
    lines.push("Permanently delete these eligible temporary files?".to_owned());
    let buttons = [
        Button::new("&Yes").focused(answer == Confirm::Yes),
        Button::new("&No")
            .default_button()
            .focused(answer == Confirm::No),
    ];
    dialog::render_warning(frame, tracker, TITLE, &lines, &buttons);
}

fn details(plan: &TempPlan) -> Vec<String> {
    vec![
        format!(
            "Temporary folder: {}",
            crate::path_display::label(plan.root.path())
        ),
        "Only regular files older than 7 days are eligible.".to_owned(),
        format!("Eligible files: {} ({})", plan.files.len(), plan.bytes),
        format!("Skipped during scan: {}", plan.skipped),
        "Recent files, links and all directories are kept.".to_owned(),
    ]
}

fn message(
    frame: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    text: &str,
    button: &str,
) {
    dialog::render_tracked(
        frame,
        tracker,
        TITLE,
        Ground::Prompt,
        &[Row::Text(text)],
        &[Button::new(button).default_button().focused(true)],
    );
}

fn finished(frame: &mut Frame<'_>, tracker: &mut crate::mouse::DialogMouse, outcome: &TempOutcome) {
    let lines = [
        format!(
            "Removed {} in {} temporary files",
            outcome.freed, outcome.removed
        ),
        format!(
            "Skipped: {}; could not remove: {}",
            outcome.skipped, outcome.failures
        ),
        "The temporary folder and its directories were preserved.".to_owned(),
    ];
    let rows: Vec<_> = lines.iter().map(|line| Row::Plain(line)).collect();
    dialog::render_tracked(
        frame,
        tracker,
        TITLE,
        Ground::Prompt,
        &rows,
        &[Button::new("&Close").default_button().focused(true)],
    );
}

#[cfg(test)]
mod tests;
