//! Purpose: apply a language server's event to the editor it belongs to — state, diagnostics, replies — and ignore replies meant for an older context.

use crate::lsp::manager::{Envelope, Event};
use crate::models::{App, ServerState};

pub(super) fn apply_event(app: &mut App, Envelope(owner, event): Envelope) -> bool {
    let trust = app.config.editor.lsp_trust;
    let Some(editor) = app.get_editor_mut().filter(|editor| editor.lsp.id == owner) else {
        return false;
    };
    match editor_event(editor, event, trust) {
        EventEffect::Ignored => false,
        EventEffect::Changed => true,
        EventEffect::Definition(target) => {
            crate::ops::editor_lsp::definition(app, &target);
            true
        }
    }
}

enum EventEffect {
    Ignored,
    Changed,
    Definition(crate::lsp::features::Definition),
}

fn editor_event(
    editor: &mut crate::models::Editor,
    event: Event,
    trust: crate::config::LspTrust,
) -> EventEffect {
    let target = match event {
        Event::TrustRequired(root) => {
            workspace_trust(editor, root, trust);
            None
        }
        Event::State(state) => {
            editor.lsp.state = state;
            None
        }
        Event::InstallationFailed(message) => {
            editor.lsp.state = ServerState::InstallationFailed;
            editor.notice = Some(message.into());
            None
        }
        Event::Diagnostics(stamp, diagnostics) if stamp == editor.lsp_stamp() => {
            editor.lsp.diagnostics = diagnostics;
            None
        }
        Event::Reply(query, Ok(reply)) if editor.accepts_lsp_reply(query) => {
            editor.notice = None;
            editor.show_lsp_reply(query, reply)
        }
        Event::Reply(query, Err(error)) if editor.accepts_lsp_reply(query) => {
            editor.notice = Some(format!("Language-server request failed ({})", error.code));
            None
        }
        _ => return EventEffect::Ignored, // Stale replies belong to an older editor context.
    };
    target.map_or(EventEffect::Changed, EventEffect::Definition)
}

fn workspace_trust(
    editor: &mut crate::models::Editor,
    root: std::path::PathBuf,
    trust: crate::config::LspTrust,
) {
    match trust {
        crate::config::LspTrust::Prompt => editor.lsp.ask_trust(root),
        crate::config::LspTrust::Automatic => {
            editor.lsp.popup = None;
            editor.lsp.state = ServerState::Starting;
            editor.lsp.intent = crate::models::lsp::Intent::Trust;
        }
    }
}
