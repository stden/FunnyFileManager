//! Purpose: Re-lex dirty lines and stop once the outgoing continuation state converges.

use super::super::{hash_of, Continuation, Highlighter, LineSpans};
use super::{Document, LineState};

impl LineState {
    pub(in crate::highlight::cache) fn clean(line: &str, after: Continuation) -> Self {
        Self {
            hash: hash_of(line),
            after,
        }
    }

    /// Store `!hash_of(line)` so reconciliation sees a mismatch and re-lexes
    /// even when the line's real hash is `0` (e.g. zero-padded input).
    pub(in crate::highlight::cache) fn stale(line: &str, after: Continuation) -> Self {
        Self {
            hash: !hash_of(line),
            after,
        }
    }

    pub(in crate::highlight::cache) fn matches_hash(self, line: &str) -> bool {
        self.hash == hash_of(line)
    }
}

#[derive(Clone, Copy)]
struct ReconcilePass {
    incoming: Continuation,
    prev_incoming: Continuation,
    visible_top: usize,
    visible_end: usize,
    dirty_until: usize,
}

struct Slot<'a> {
    idx: usize,
    line: &'a str,
    spans: &'a mut Option<LineSpans>,
    state: &'a mut LineState,
}

enum LineOutcome {
    NextLine,
    Converged,
    TruncateOffscreenTail,
}

impl Document {
    pub(super) fn reconcile_in_place(
        &mut self,
        lines: &[String],
        visible_top: usize,
        visible_end: usize,
        initial: Continuation,
        dirty: Option<(usize, usize)>,
    ) {
        let limit = self.spans.len().min(lines.len());
        let (start, dirty_until) = bounds(dirty, limit);
        let incoming = self.state_before(start).unwrap_or(initial);
        let mut pass = ReconcilePass {
            incoming,
            prev_incoming: incoming,
            visible_top,
            visible_end,
            dirty_until,
        };
        if let Some(idx) = self.walk(lines, start, limit, &mut pass) {
            self.spans.truncate(idx);
            self.states.truncate(idx);
        }
    }

    pub(in crate::highlight::cache) fn splice_lines(
        &mut self,
        start: usize,
        old_end: usize,
        added: &[String],
    ) {
        self.line_count = self.line_count - (old_end - start) + added.len();
        self.revision = None;
        if let Some(init) = self.initial.filter(|_| old_end < self.spans.len()) {
            let state_at = self.state_before(start).unwrap_or(init);
            self.spans
                .splice(start..old_end, std::iter::repeat_n(None, added.len()));
            self.states.splice(
                start..old_end,
                added.iter().map(|line| LineState::stale(line, state_at)),
            );
        } else if start < self.spans.len() {
            self.spans.truncate(start);
            self.states.truncate(start);
        }
    }

    fn walk(
        &mut self,
        lines: &[String],
        start: usize,
        limit: usize,
        pass: &mut ReconcilePass,
    ) -> Option<usize> {
        for (idx, ((line, state_slot), span_slot)) in lines
            .iter()
            .zip(self.states.iter_mut())
            .zip(self.spans.iter_mut())
            .enumerate()
            .take(limit)
            .skip(start)
        {
            let mut slot = Slot {
                idx,
                line,
                spans: span_slot,
                state: state_slot,
            };
            match pass.step_line(&mut self.engine, &mut slot) {
                LineOutcome::NextLine => {}
                LineOutcome::Converged => return None,
                LineOutcome::TruncateOffscreenTail => return Some(idx),
            }
        }
        None
    }
}

fn bounds(dirty: Option<(usize, usize)>, limit: usize) -> (usize, usize) {
    let (start, dirty_until) = dirty.unwrap_or((0, usize::MAX));
    (start.min(limit), dirty_until)
}

impl ReconcilePass {
    fn step_line(&mut self, engine: &mut Highlighter, slot: &mut Slot<'_>) -> LineOutcome {
        let prev = *slot.state;
        if self.incoming == self.prev_incoming && prev.matches_hash(slot.line) {
            if slot.idx >= self.dirty_until {
                return LineOutcome::Converged;
            }
            self.incoming = prev.after;
            self.prev_incoming = prev.after;
            return LineOutcome::NextLine;
        }
        if slot.idx >= self.visible_end {
            return LineOutcome::TruncateOffscreenTail;
        }
        self.relex(engine, slot);
        LineOutcome::NextLine
    }

    fn relex(&mut self, engine: &mut Highlighter, slot: &mut Slot<'_>) {
        let old_after = slot.state.after;
        engine.resume(self.incoming);
        if slot.idx < self.visible_top {
            engine.advance_only(slot.line);
            *slot.spans = None;
        } else {
            *slot.spans = Some(super::lex_line(engine, slot.line));
        }
        let new_after = engine.continuation().unwrap_or(self.incoming);
        *slot.state = LineState::clean(slot.line, new_after);
        self.incoming = new_after;
        self.prev_incoming = old_after;
    }
}
