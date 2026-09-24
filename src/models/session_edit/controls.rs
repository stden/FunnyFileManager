//! Purpose: The cursor over the session dialog's controls, and what typing,
//! Backspace and the arrows do to the control it is on.

use super::{AuthKind, Protocol, SessionEdit, SessionField};
use crate::cycle;

impl SessionEdit {
    /// Tab / Shift+Tab: the next control, passing over the key file field
    /// while the login is not by key file.
    pub fn tab(&mut self, forward: bool) {
        self.field = cycle::step(&SessionField::ALL, self.field, forward);
        if !self.is_enabled(self.field) {
            self.field = cycle::step(&SessionField::ALL, self.field, forward);
        }
    }

    /// Whether a control takes the cursor: all do but the key file field
    /// while the login is by password or agent.
    #[must_use]
    pub fn is_enabled(&self, field: SessionField) -> bool {
        field != SessionField::KeyFile || self.auth == AuthKind::KeyFile
    }

    /// A typed character, into the text field with the cursor.
    pub fn push(&mut self, ch: char) {
        if let Some(text) = self.text_mut() {
            text.push(ch);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(text) = self.text_mut() {
            text.pop();
        }
    }

    /// Left / Right / Space on a radio row: the next or previous choice. A
    /// port still at the old protocol's default follows the new one, so
    /// SFTP → FTP moves 22 to 21 without being retyped.
    pub fn cycle_choice(&mut self, forward: bool) {
        match self.field {
            SessionField::Protocol => {
                let previous = self.protocol;
                self.protocol = cycle::step(&Protocol::ALL, previous, forward);
                if self.port.trim() == previous.default_port().to_string() {
                    self.port = self.protocol.default_port().to_string();
                }
            }
            SessionField::Auth => self.auth = cycle::step(&AuthKind::ALL, self.auth, forward),
            _ => {}
        }
    }

    #[must_use]
    pub const fn on_cancel(&self) -> bool {
        matches!(self.field, SessionField::Cancel)
    }

    fn text_mut(&mut self) -> Option<&mut String> {
        match self.field {
            SessionField::Name => Some(&mut self.name),
            SessionField::Host => Some(&mut self.host),
            SessionField::Port => Some(&mut self.port),
            SessionField::User => Some(&mut self.user),
            SessionField::KeyFile => Some(&mut self.key_file),
            SessionField::RemoteDir => Some(&mut self.remote_dir),
            SessionField::Protocol
            | SessionField::Auth
            | SessionField::Ok
            | SessionField::Cancel => None,
        }
    }
}
