//! Purpose: `App`'s status seam — set an outcome, read its text, and expire it once the cursor moves on.

use crate::models::App;

use super::{Status, StatusCursor, StatusDisplay};

impl App {
    /// Set the status-bar message from a typed [`Status`] outcome.
    ///
    /// This is the one place handlers should go through to report an outcome —
    /// they name *what happened*; [`Status`] decides *how it reads*.
    pub fn set_status(&mut self, status: Status) {
        self.status = status;
        // A message that has never been drawn has no cursor yet; the next
        // frame gives it one. Without this, a message produced by an operation
        // that also moved the cursor — F8 deletes the entry under it — would
        // be measured against the previous message's cursor and cleared before
        // anyone had seen it.
        self.status_display = StatusDisplay::Pending;
    }

    /// Report how a program FFM handed the terminal to ended: in the status
    /// bar, and as the closing line of that run in the Ctrl+O transcript, in
    /// the same words. Plain command output and a program's exit read as
    /// output; anything else (an error, a cancel) as an error line.
    pub(crate) fn record_run_outcome(&mut self, status: Status) {
        let text = status.to_string();
        if matches!(
            status,
            Status::CommandOutput(_) | Status::ProgramExited { .. }
        ) {
            self.console.record_output(&text);
        } else {
            self.console.record_error(&text);
        }
        self.set_status(status);
    }

    /// The message the active panel's info row should show, or `None` when
    /// there is no news and the entry's own name belongs there.
    ///
    /// Neither `Cleared` nor `KeyHints` is an outcome: the first has nothing
    /// to say and the second repeats the F-key bar two rows below it. This is
    /// the one definition of "has news", shared by the renderer and by
    /// [`Self::expire_status_on_move`], which must not drift apart.
    #[must_use]
    pub fn status_news(&self) -> Option<String> {
        if matches!(self.status_display, StatusDisplay::Expired)
            || matches!(self.status, Status::Cleared | Status::KeyHints)
        {
            return None;
        }
        let message = self.status_text();
        (!message.is_empty()).then_some(message)
    }

    fn cursor_now(&self) -> StatusCursor {
        let panel = self.get_active_panel_ref();
        StatusCursor {
            side: self.active_panel,
            directory: panel.path.clone(),
            row: panel.shown_cursor(),
        }
    }

    /// Tie the message on screen to the cursor it was produced for, and drop
    /// it once the cursor has moved away.
    ///
    /// Far puts the entry's name back in the info row as soon as the user
    /// navigates; before this, the last operation's message sat there hiding
    /// the name until some other operation replaced it.
    ///
    /// Called once per drawn frame, immediately before the draw. A message set
    /// since the last frame has no cursor yet, so this call gives it the
    /// cursor *this* frame draws and the comparison cannot fail here — only a
    /// later frame, with the cursor somewhere else, can clear it. That is what
    /// guarantees every message is seen at least once, whatever the operation
    /// did to the selection on its way out.
    pub fn expire_status_on_move(&mut self) {
        // While a quick search owns the info row the message is not on show,
        // so it has neither been seen nor been moved away from. Tying it to a
        // cursor here would expire it on the next arrow key and it would never
        // reach the screen at all.
        if self.quick_search.query().is_some() {
            return;
        }
        match &self.status_display {
            StatusDisplay::Drawn(drawn_at) if drawn_at.is_current(self) => {}
            // Expired for the info row only. `status` itself stays: the
            // command prompt colours its bullet from the same field, and a
            // failed command must not start reporting success because the
            // cursor moved.
            StatusDisplay::Drawn(_) => self.status_display = StatusDisplay::Expired,
            StatusDisplay::Pending => {
                if self.status_news().is_some() {
                    self.status_display = StatusDisplay::Drawn(self.cursor_now());
                }
            }
            StatusDisplay::Expired => {}
        }
    }

    /// The status bar's current line.
    ///
    /// Rendered on demand rather than stored: keeping the `Status` means a test
    /// can match the outcome instead of a substring, and the formatting stays
    /// in one place.
    #[must_use]
    pub fn status_text(&self) -> String {
        self.status.to_string()
    }
}
