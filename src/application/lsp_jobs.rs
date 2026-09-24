//! Purpose: Debounce editor snapshots and map nonblocking LSP events onto the UI state.

use crate::{
    lsp::{
        manager::{Command, Manager},
        positions::Position,
        types::{Query, Stamp},
    },
    models::{lsp::Intent, App, ServerState},
};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

mod events;

use events::apply_event;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Context {
    stamp: Stamp,
    position: Option<Position>,
    saved: u64,
}

#[derive(Default)]
pub(super) struct EditorLspController {
    manager: Option<Manager>,
    observed: Option<Context>,
    sent: Option<(Stamp, u64)>,
    changed: Option<Instant>,
    pending: VecDeque<Command>,
}

impl EditorLspController {
    pub fn pump(&mut self, app: &mut App) -> bool {
        self.flush();
        let mut redraw = self.receive(app);
        if self.pending.is_empty() {
            redraw |= self.observe(app, Instant::now());
        }
        self.flush();
        redraw
    }

    fn observe(&mut self, app: &mut App, now: Instant) -> bool {
        let server = app
            .get_editor()
            .and_then(|editor| app.config.lsp.for_path(&editor.path))
            .map(|(language, settings)| (language.to_owned(), settings.clone()));
        let Some((language, settings)) = server else {
            self.close();
            return disabled_notice(app);
        };
        let Some(editor) = app.get_editor_mut() else {
            return false;
        };
        let context = Context::from_editor(editor);
        if self
            .observed
            .is_none_or(|old| old.stamp.id != context.stamp.id)
            || editor.lsp.intent == Intent::Restart
        {
            self.open_editor(editor);
            if self.manager.is_none() {
                return true;
            }
            self.pending
                .push_back(Command::Open(language, settings, editor.lsp_snapshot()));
            self.sent = Some((context.stamp, context.saved));
            self.observed = Some(context);
            return true;
        }
        self.update(editor, context, now);
        false
    }

    fn update(&mut self, editor: &mut crate::models::Editor, context: Context, now: Instant) {
        if editor.lsp.intent == Intent::Trust {
            editor.lsp.intent = Intent::None;
            self.pending.push_back(Command::Trust(context.stamp.id));
        }
        if self.observed != Some(context) {
            self.pending.push_back(Command::Cancel);
            self.changed = Some(now);
            self.observed = Some(context);
        }
        let requested = matches!(editor.lsp.intent, Intent::Query(_));
        let saved = self.sent.is_some_and(|(_, saved)| saved != context.saved);
        let settled = self
            .changed
            .is_some_and(|changed| now.duration_since(changed) >= Duration::from_millis(120));
        if (requested || saved || settled) && self.sent != Some((context.stamp, context.saved)) {
            self.pending
                .push_back(Command::Update(editor.lsp_snapshot()));
            self.sent = Some((context.stamp, context.saved));
        }
        if let Some(command) = requested_query(editor, context) {
            self.pending.push_back(command);
        }
    }

    fn open_editor(&mut self, editor: &mut crate::models::Editor) {
        editor.lsp.intent = Intent::None;
        editor.lsp.state = ServerState::Starting;
        if self.manager.is_none() {
            if let Ok(manager) = Manager::start() {
                self.manager = Some(manager);
            } else {
                editor.lsp.state = ServerState::Failed;
                crate::diagnostics::log_error("Cannot start LSP worker");
            }
        }
    }

    fn close(&mut self) {
        if self.observed.take().is_some() {
            self.pending.push_back(Command::Close);
        }
        self.sent = None;
        self.changed = None;
    }

    fn flush(&mut self) {
        let Some(manager) = &self.manager else {
            return;
        };
        while let Some(command) = self.pending.pop_front() {
            if let Err(command) = manager.send(command) {
                self.pending.push_front(*command);
                break;
            }
        }
    }

    fn receive(&self, app: &mut App) -> bool {
        let Some(manager) = &self.manager else {
            return false;
        };
        let mut changed = false;
        while let Some(envelope) = manager.poll() {
            changed |= apply_event(app, envelope);
        }
        changed
    }
}

impl Context {
    fn from_editor(editor: &crate::models::Editor) -> Self {
        Self {
            stamp: editor.lsp_stamp(),
            position: editor.lsp_position(),
            saved: editor.lsp.saved,
        }
    }
}

fn disabled_notice(app: &mut App) -> bool {
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

fn requested_query(editor: &mut crate::models::Editor, context: Context) -> Option<Command> {
    let Intent::Query(feature) = editor.lsp.intent else {
        return None;
    };
    editor.lsp.intent = Intent::None;
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

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_session;
