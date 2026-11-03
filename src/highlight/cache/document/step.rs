//! Purpose: re-lex a dirty span and stop once the continuation matches again.

use super::super::{hash_of, Continuation, Highlighter, LineSpans};
use super::Document;

struct Pass<'a> {
    incoming: &'a mut Continuation,
    old_incoming: &'a mut Continuation,
    top: usize,
}

struct Slot<'a> {
    idx: usize,
    line: &'a str,
    spans: &'a mut Option<LineSpans>,
    state: &'a mut (u64, Continuation),
}

enum Step {
    Done,
    Cut,
    Continue,
}

impl Document {
    pub(super) fn reconcile_in_place(
        &mut self,
        lines: &[String],
        top: usize,
        end: usize,
        initial: Continuation,
        dirty: Option<(usize, usize)>,
    ) {
        let limit = self.spans.len().min(lines.len());
        let (start, until) = bounds(dirty, limit);
        let mut incoming = self.state_before(start).unwrap_or(initial);
        let mut old_incoming = incoming;
        let mut pass = Pass {
            incoming: &mut incoming,
            old_incoming: &mut old_incoming,
            top,
        };
        if let Some(idx) = self.walk(lines, start, limit, end, until, &mut pass) {
            self.spans.truncate(idx);
            self.states.truncate(idx);
        }
    }

    fn walk(
        &mut self,
        lines: &[String],
        start: usize,
        limit: usize,
        end: usize,
        until: usize,
        pass: &mut Pass<'_>,
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
            match consider(&mut self.engine, until, end, &mut slot, pass) {
                Step::Done => return None,
                Step::Cut => return Some(idx),
                Step::Continue => {}
            }
        }
        None
    }
}

fn bounds(dirty: Option<(usize, usize)>, limit: usize) -> (usize, usize) {
    let (start, until) = dirty.unwrap_or((0, usize::MAX));
    (start.min(limit), until)
}

fn consider(
    engine: &mut Highlighter,
    until: usize,
    end: usize,
    slot: &mut Slot<'_>,
    pass: &mut Pass<'_>,
) -> Step {
    let (old_hash, old_after) = *slot.state;
    if *pass.incoming == *pass.old_incoming && hash_of(slot.line) == old_hash {
        if slot.idx >= until {
            return Step::Done;
        }
        *pass.incoming = old_after;
        *pass.old_incoming = old_after;
        return Step::Continue;
    }
    if slot.idx >= end {
        return Step::Cut;
    }
    relex(engine, slot, pass);
    Step::Continue
}

fn relex(engine: &mut Highlighter, slot: &mut Slot<'_>, pass: &mut Pass<'_>) {
    let old_after = slot.state.1;
    engine.resume(*pass.incoming);
    if slot.idx < pass.top {
        engine.advance_only(slot.line);
        *slot.spans = None;
    } else {
        *slot.spans = Some(super::lex_line(engine, slot.line));
    }
    let new_after = engine.continuation().unwrap_or(*pass.incoming);
    *slot.state = (hash_of(slot.line), new_after);
    *pass.incoming = new_after;
    *pass.old_incoming = old_after;
}
