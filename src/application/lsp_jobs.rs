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

use events::{apply_event, disabled_notice, requested_query, Context, CursorKey};

#[derive(Default)]
pub(super) struct EditorLspController {
    manager: Option<Manager>,
    observed: Option<Context>,
    cursor_key: Option<CursorKey>,
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
        let needs_open = app.get_editor().is_some_and(|ed| {
            self.observed.is_none_or(|old| old.stamp.id != ed.lsp.id)
                || ed.lsp.intent == Intent::Restart
        });
        let server = app
            .get_editor()
            .and_then(|ed| app.config.lsp.for_path(&ed.path))
            .map(|(lang, cfg)| needs_open.then(|| (lang.to_owned(), cfg.clone())));
        let (Some(open_cfg), Some(editor)) = (server, app.get_editor_mut()) else {
            self.close();
            return disabled_notice(app);
        };
        if let Some((lang, cfg)) = open_cfg {
            return self.open_session(editor, lang, cfg);
        }
        if let Some(key) = Context::next_cursor_key(editor, self.cursor_key, self.sent) {
            self.cursor_key = Some(key);
            let context = Context::from_editor(editor);
            self.update(editor, context, now);
        }
        false
    }

    fn open_session(
        &mut self,
        ed: &mut crate::models::Editor,
        lang: String,
        cfg: crate::config::ServerSettings,
    ) -> bool {
        let (ctx, restart) = (Context::from_editor(ed), ed.lsp.intent == Intent::Restart);
        self.open_editor(ed);
        if self.manager.is_some() {
            self.pending.extend(restart.then_some(Command::Restart));
            self.pending
                .push_back(Command::Open(lang, cfg, ed.lsp_snapshot()));
            (self.sent, self.observed) = (Some((ctx.stamp, ctx.saved)), Some(ctx));
            self.cursor_key = Some(Context::cursor_key(ed));
        }
        true
    }

    fn update(&mut self, editor: &mut crate::models::Editor, context: Context, now: Instant) {
        if editor.lsp.intent == Intent::Trust {
            editor.lsp.intent = Intent::None;
            self.pending.push_back(Command::Trust(context.stamp.id));
        }
        if self.observed != Some(context) {
            if !self.pending.iter().any(|c| matches!(c, Command::Cancel)) {
                self.pending.push_back(Command::Cancel);
            }
            (self.changed, self.observed) = (Some(now), Some(context));
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
            if let Some(manager) = start_manager() {
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
        self.cursor_key = None;
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
thread_local! {
    pub(super) static FORCE_SPAWN_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn start_manager() -> Option<Manager> {
    #[cfg(test)]
    if FORCE_SPAWN_FAILURE.get() {
        return None;
    }
    Manager::start().ok()
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_mailbox;
#[cfg(test)]
mod tests_session;
