//! Purpose: remember which lines an edit touched, so the next paint can skip
//! hashing the unchanged prefix of a large buffer.

use super::Editor;

impl Editor {
    /// Record that buffer lines `from..until` changed. Later edits widen the span.
    pub(super) fn note_dirty(&self, from: usize, until: usize) {
        let until = until.max(from.saturating_add(1));
        let next = match self.dirty_span.get() {
            Some((start, end)) => (start.min(from), end.max(until)),
            None => (from, until),
        };
        self.dirty_span.set(Some(next));
    }

    /// The span noted since the last paint, if every edit went through the editor.
    pub(crate) fn take_dirty_span(&self) -> Option<(usize, usize)> {
        self.dirty_span.take()
    }
}
