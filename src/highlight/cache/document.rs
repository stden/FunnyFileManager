//! Purpose: Incremental per-line span storage and continuation-state convergence for `HighlightCache`.

use std::path::Path;

use super::{Continuation, Document, Highlighter, LineSpans, Span};

mod step;

#[derive(Debug, Clone, Copy)]
pub(super) struct LineState {
    pub(super) hash: u64,
    pub(super) after: Continuation,
}

fn lex_line(engine: &mut Highlighter, line: &str) -> LineSpans {
    let raw = engine.line(line);
    let mut spans = Vec::with_capacity(raw.len());
    let mut start = 0;
    for (text, token) in raw {
        let end = start + text.len();
        if let Some(last) = spans
            .last_mut()
            .filter(|last: &&mut Span| last.token == token && last.end == start)
        {
            last.end = end;
        } else {
            spans.push(Span { start, end, token });
        }
        start = end;
    }
    spans
}

impl Document {
    pub(super) fn new(
        path: &Path,
        lines: &[String],
        #[cfg(test)] fingerprint: u64,
        #[cfg(not(test))] _fingerprint: u64,
        revision: Option<u64>,
    ) -> Self {
        let engine = Highlighter::for_file(path, lines);
        let initial = engine.continuation();
        Self {
            path: path.to_path_buf(),
            #[cfg(test)]
            fingerprint,
            revision,
            line_count: lines.len(),
            engine,
            initial,
            states: Vec::new(),
            spans: Vec::new(),
        }
    }

    pub(super) fn reconcile(
        &mut self,
        lines: &[String],
        top: usize,
        end: usize,
        initial: Continuation,
        dirty: Option<(usize, usize)>,
    ) {
        if lines.len() != self.line_count {
            self.shift_line_count(lines, initial, dirty.map(|(from, _)| from));
        }
        self.reconcile_in_place(lines, top, end, initial, dirty);
        self.ensure_window(lines, top, end);
    }

    fn shift_line_count(&mut self, lines: &[String], initial: Continuation, hint: Option<usize>) {
        let prefix = self.unchanged_prefix(lines, hint);
        if prefix < self.states.len() {
            if lines.len() > self.line_count {
                self.splice_inserted_lines(lines, prefix, initial);
            } else {
                self.drain_removed_lines(lines, prefix, initial);
            }
        }
        self.spans.truncate(lines.len());
        self.states.truncate(lines.len());
        self.line_count = lines.len();
    }

    /// Lines `0..skip` are known unchanged, so only the tail is hashed.
    fn unchanged_prefix(&self, lines: &[String], hint: Option<usize>) -> usize {
        let skip = hint.unwrap_or(0).min(self.states.len());
        let matched = self
            .states
            .iter()
            .skip(skip)
            .zip(lines.iter().skip(skip))
            .take_while(|(state, line)| state.matches_hash(line))
            .count();
        skip + matched
    }

    fn splice_inserted_lines(&mut self, lines: &[String], prefix: usize, initial: Continuation) {
        let added = lines.len() - self.line_count;
        let state_at_prefix = self.state_before(prefix).unwrap_or(initial);
        let inserted = lines.get(prefix..prefix + added).unwrap_or_default();
        self.spans
            .splice(prefix..prefix, std::iter::repeat_n(None, added));
        self.states.splice(
            prefix..prefix,
            inserted
                .iter()
                .map(|line| LineState::stale(line, state_at_prefix)),
        );
    }

    fn drain_removed_lines(&mut self, lines: &[String], prefix: usize, initial: Continuation) {
        let removed = (self.line_count - lines.len()).min(self.states.len() - prefix);
        let old_in = self.state_before(prefix + removed).unwrap_or(initial);
        let new_in = self.state_before(prefix).unwrap_or(initial);
        self.spans.drain(prefix..prefix + removed);
        self.states.drain(prefix..prefix + removed);
        if old_in != new_in {
            if let Some((slot, line)) = self.states.get_mut(prefix).zip(lines.get(prefix)) {
                *slot = LineState::stale(line, slot.after);
            }
        }
    }

    fn state_before(&self, idx: usize) -> Option<Continuation> {
        incoming_state(self.initial, &self.states, idx)
    }

    pub(super) fn ensure_window(&mut self, lines: &[String], top: usize, end: usize) {
        if !self.engine.is_active() {
            return;
        }
        let lex_from = if self.initial.is_some() { top } else { 0 };
        let missing = end.saturating_sub(self.spans.len());
        self.states.reserve(missing);
        self.spans.reserve(missing);
        for (idx, line) in lines.iter().enumerate().take(end).skip(self.spans.len()) {
            if let Some(prev) = self.state_before(idx) {
                self.engine.resume(prev);
            }
            let spans = if idx < lex_from {
                self.engine.advance_only(line);
                None
            } else {
                Some(lex_line(&mut self.engine, line))
            };
            if let Some(after) = self.engine.continuation() {
                self.states.push(LineState::clean(line, after));
            }
            self.spans.push(spans);
        }
        self.fill_window_spans(lines, lex_from, end);
    }

    fn fill_window_spans(&mut self, lines: &[String], lex_from: usize, end: usize) {
        for (idx, slot) in self.spans.iter_mut().enumerate().take(end).skip(lex_from) {
            if slot.is_none() {
                let prev = incoming_state(self.initial, &self.states, idx);
                *slot = compute_span(&mut self.engine, prev, lines.get(idx));
            }
        }
    }
}

fn incoming_state(
    init: Option<Continuation>,
    states: &[LineState],
    idx: usize,
) -> Option<Continuation> {
    let first = init?;
    idx.checked_sub(1)
        .map_or(Some(first), |prev| states.get(prev).map(|s| s.after))
}

fn compute_span(
    engine: &mut Highlighter,
    prev: Option<Continuation>,
    line: Option<&String>,
) -> Option<LineSpans> {
    engine.resume(prev?);
    Some(lex_line(engine, line?))
}
