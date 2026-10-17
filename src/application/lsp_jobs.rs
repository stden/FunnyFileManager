//! Purpose: Debounce editor snapshots and map nonblocking LSP events onto the UI state.

#[cfg(test)]
use crate::lsp::{positions::Position, types::Query};
use crate::{
    lsp::{
        manager::{Command, Manager, SendError},
        types::Stamp,
    },
    models::{lsp::Intent, App, ServerState},
};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

mod events;

use events::{apply_event, disabled_notice, requested_query, Context};

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
        let mut redraw = self.receive(app);
        self.flush(app);
        redraw |= self.observe(app, Instant::now());
        self.flush(app);
        redraw | self.receive(app)
    }

    fn observe(&mut self, app: &mut App, now: Instant) -> bool {
        let server = app
            .get_editor()
            .and_then(|editor| app.config.lsp.for_path(&editor.path))
            .map(|(language, settings)| (language.to_owned(), settings.clone()));
        let (Some((language, settings)), Some(editor)) = (server, app.get_editor_mut()) else {
            self.close();
            return disabled_notice(app);
        };
        let context = Context::from_editor(editor);
        let restart = editor.lsp.intent == Intent::Restart;
        if self
            .observed
            .is_none_or(|old| old.stamp.id != context.stamp.id)
            || restart
        {
            self.open_editor(editor);
            if self.manager.is_some() {
                self.pending.extend(restart.then_some(Command::Restart));
                self.pending
                    .push_back(Command::Open(language, settings, editor.lsp_snapshot()));
                self.sent = Some((context.stamp, context.saved));
                self.observed = Some(context);
            }
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
            if !self
                .pending
                .iter()
                .any(|cmd| matches!(cmd, Command::Cancel))
            {
                self.pending.push_back(Command::Cancel);
            }
            self.changed = Some(now);
            self.observed = Some(context);
        }
        let requested = matches!(editor.lsp.intent, Intent::Query(_));
        let saved = self.sent.is_some_and(|(_, saved)| saved != context.saved);
        let settled = self
            .changed
            .is_some_and(|t| now.duration_since(t) >= Duration::from_millis(120));
        if (requested || saved || settled) && self.sent != Some((context.stamp, context.saved)) {
            self.pending
                .retain(|cmd| !matches!(cmd, Command::Update(_)));
            self.pending
                .push_back(Command::Update(editor.lsp_snapshot()));
            self.sent = Some((context.stamp, context.saved));
            self.pending
                .extend(events::auto_query(editor, context, settled && !requested));
        }
        self.pending.extend(requested_query(editor, context));
    }

    fn open_editor(&mut self, editor: &mut crate::models::Editor) {
        self.pending.clear();
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
        self.pending.clear();
        if self.observed.take().is_some() {
            self.pending.push_back(Command::Close);
        }
        self.sent = None;
        self.changed = None;
    }

    fn flush(&mut self, app: &mut App) {
        let Some(manager) = &self.manager else {
            return;
        };
        let mut disconnected = false;
        while let Some(command) = self.pending.pop_front() {
            match manager.send(command) {
                Ok(()) => {}
                Err(SendError::Full(command)) => {
                    self.pending.push_front(*command);
                    break;
                }
                Err(SendError::Disconnected(_)) => {
                    disconnected = true;
                    break;
                }
            }
        }
        if disconnected {
            self.pending.clear();
            self.manager = None;
            if let Some(editor) = app.get_editor_mut() {
                editor.lsp.state = ServerState::Failed;
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

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_session;
