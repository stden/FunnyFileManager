//! Purpose: apply a language server's event to the editor it belongs to — state, diagnostics, replies — and ignore replies meant for an older context.

use crate::lsp::{
    manager::{Command, Envelope, Event},
    positions::Position,
    types::{Query, Stamp},
};
use crate::models::{lsp::Intent, App, ServerState};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Context {
    pub stamp: Stamp,
    pub position: Option<Position>,
    pub saved: u64,
}

impl Context {
    pub(super) fn from_editor(editor: &crate::models::Editor) -> Self {
        Self {
            stamp: editor.lsp_stamp(),
            position: editor.lsp_position(),
            saved: editor.lsp.saved,
        }
    }
}

pub(super) fn disabled_notice(app: &mut App) -> bool {
    let Some(editor) = app
        .get_editor_mut()
        .filter(|editor| editor.lsp.intent != Intent::None)
    else {
        return false;
    };
    editor.lsp.intent = Intent::None;
    editor.notice = Some("LSP is disabled for this file".into());
    true
}

pub(super) fn auto_query(
    editor: &mut crate::models::Editor,
    context: Context,
    settled: bool,
) -> Option<Command> {
    if !settled || !editor.lsp.auto_complete {
        return None;
    }
    editor.lsp.auto_complete = false;
    if editor.lsp.state != ServerState::Ready {
        return None;
    }
    let position = context.position?;
    editor.lsp.silent_completion = true;
    Some(Command::Query(Query {
        stamp: context.stamp,
        position,
        feature: crate::lsp::types::Feature::Completion,
    }))
}

pub(super) fn requested_query(
    editor: &mut crate::models::Editor,
    context: Context,
) -> Option<Command> {
    let Intent::Query(feature) = editor.lsp.intent else {
        return None;
    };
    editor.lsp.intent = Intent::None;
    editor.lsp.auto_complete = false;
    editor.lsp.silent_completion = false;
    if matches!(
        editor.lsp.state,
        ServerState::Unavailable
            | ServerState::Failed
            | ServerState::InstallationFailed
            | ServerState::Stopped
            | ServerState::Untrusted
    ) {
        editor.notice = Some(editor.lsp.state.label().into());
        return None;
    }
    let position = context.position?;
    editor.notice = Some("Requesting language-server information".into());
    Some(Command::Query(Query {
        stamp: context.stamp,
        position,
        feature,
    }))
}

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
            editor.lsp.intent = Intent::Trust;
        }
    }
}
