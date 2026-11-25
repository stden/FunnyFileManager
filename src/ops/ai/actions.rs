//! Purpose: Interactive AI assistant actions (prompting, switching providers, inserting commands).

use std::fmt::Write;
use std::sync::Arc;

use crate::models::services::ai::{AiDialog, AiRequestId};
use crate::models::{App, CommandLine, Overlay};
use crate::status::Status;
use crate::values::ai::{AiError, AiProvider, AiResponse, WorkspaceContext};

/// Snapshot active/opposite panel directories and selections for AI prompt context.
#[must_use]
pub fn context_for(app: &App) -> WorkspaceContext {
    let active = app.get_active_panel_ref();
    let opposite = app.get_inactive_panel_ref();
    let selected = active
        .selected_path()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
    let marked = active
        .marked
        .iter()
        .map(|n| n.as_str().to_owned())
        .collect();
    WorkspaceContext {
        active_dir: active.path.display().to_string(),
        opposite_dir: opposite.path.display().to_string(),
        selected,
        marked,
        os: std::env::consts::OS,
    }
}

/// Submit the non-blank line in `AiDialog::prompt` to the selected AI provider.
pub fn submit_prompt(app: &mut App) {
    let context = context_for(app);
    let Overlay::Ai(dialog) = &mut app.overlay else {
        return;
    };
    if !dialog.is_busy() {
        dialog.queue_prompt(context);
    }
}

/// Cycle the active AI backend (`Anthropic` -> `OpenAI` -> `Codex`).
pub fn cycle_provider(app: &mut App) {
    let Overlay::Ai(dialog) = &mut app.overlay else {
        return;
    };
    if !dialog.is_busy() {
        dialog.cycle_provider();
    }
}

/// Pre-fill the `:` command line with the last extracted shell command for user confirmation.
pub fn insert_command(app: &mut App) {
    let command = {
        let Overlay::Ai(dialog) = &app.overlay else {
            return;
        };
        dialog.last_command.clone()
    };
    let Some(cmd) = command else {
        app.set_status(Status::Error("No shell command in AI reply".to_owned()));
        return;
    };
    app.set_overlay(Overlay::CommandLine(CommandLine::from(cmd)));
    app.set_status(Status::CommandOutput(
        "AI command placed in prompt — review and press Enter to run".to_owned(),
    ));
}

/// Save the conversation transcript to `ai-transcript.md` in the active panel.
pub fn save_transcript(app: &mut App) -> std::io::Result<()> {
    let text = {
        let Overlay::Ai(dialog) = &app.overlay else {
            return Ok(());
        };
        let mut out = String::new();
        for msg in &dialog.messages {
            writeln!(out, "### {}\n\n{}\n", msg.role.prefix(), msg.text)
                .map_err(std::io::Error::other)?;
        }
        out
    };
    if text.is_empty() {
        app.set_status(Status::Error("AI conversation is empty".to_owned()));
        return Ok(());
    }
    let dir = app.get_active_panel_ref().path.clone();
    let dest = crate::utils::unique_destination(&dir, std::ffi::OsStr::new("ai-transcript.md"))?;
    crate::file_write::replace_owner_only(
        &dest,
        "ai-transcript",
        &text,
        crate::file_write::SyncPolicy::Skip,
    )?;
    drop(crate::utils::refresh_panels(app));
    app.set_status(Status::CommandOutput(format!("Saved {}", dest.display())));
    Ok(())
}

/// Clear all turns in the active AI dialog.
pub fn clear_history(app: &mut App) {
    let Overlay::Ai(dialog) = &mut app.overlay else {
        return;
    };
    dialog.clear();
}

/// Scroll AI conversation output (`delta < 0` scrolls older lines).
pub fn scroll_history(app: &mut App, delta: isize) {
    let Overlay::Ai(dialog) = &mut app.overlay else {
        return;
    };
    if delta < 0 {
        dialog.scroll = dialog.scroll.saturating_add(delta.unsigned_abs());
    } else {
        dialog.scroll = dialog.scroll.saturating_sub(delta.unsigned_abs());
    }
}

/// Switch active AI dialog to `provider` if not currently waiting on a response.
pub fn select_provider(app: &mut App, provider: AiProvider) {
    let Overlay::Ai(dialog) = &mut app.overlay else {
        return;
    };
    if !dialog.is_busy() {
        dialog.set_provider(provider);
    }
}

/// Apply the worker's AI response if the originating dialog and request ID still match.
pub fn publish(
    app: &mut App,
    generation: &Arc<()>,
    id: AiRequestId,
    reply: Result<AiResponse, AiError>,
) {
    let Overlay::Ai(dialog) = &mut app.overlay else {
        return;
    };
    if !Arc::ptr_eq(&dialog.generation, generation) || !dialog.awaits(id) {
        return;
    }
    let is_ok = reply.is_ok();
    dialog.apply_reply(reply);
    let msg = dialog.status.clone();
    let status = if is_ok {
        Status::CommandOutput(msg)
    } else {
        Status::Error(msg)
    };
    app.set_status(status);
}

#[must_use]
pub fn initial_dialog(
    app: &App,
    prompt_text: Option<&str>,
    provider_override: Option<AiProvider>,
) -> AiDialog {
    let settings = if app.live.ai == crate::values::ai::AiAccess::Network {
        crate::ai::config::load_default_settings()
    } else {
        crate::values::ai::AiSettings::default()
    };
    let mut dialog = AiDialog::new(settings);
    if let Some(provider) = provider_override {
        dialog.set_provider(provider);
    }
    if let Some(text) = prompt_text.map(str::trim).filter(|s| !s.is_empty()) {
        dialog.prompt.set_text(text);
        dialog.queue_prompt(context_for(app));
    }
    dialog
}

#[cfg(test)]
mod tests;
