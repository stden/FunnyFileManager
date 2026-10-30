//! Purpose: Incremental per-line span storage and continuation-state convergence for `HighlightCache`.

use std::path::Path;

use super::{hash_of, Continuation, Document, Highlighter, Token};

fn lex_line(engine: &mut Highlighter, line: &str) -> Vec<(usize, usize, Token)> {
    let mut start = 0;
    engine
        .line(line)
        .into_iter()
        .map(|(text, token)| {
            let end = start + text.len();
            let span = (start, end, token);
            start = end;
            span
        })
        .collect()
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

    pub(super) fn reconcile(&mut self, lines: &[String], end: usize, initial: Continuation) {
        if lines.len() != self.line_count {
            self.shift_line_count(lines, initial);
        }
        self.reconcile_in_place(lines, end, initial);
        self.extend(lines, end);
    }

    fn shift_line_count(&mut self, lines: &[String], initial: Continuation) {
        let prefix = self
            .states
            .iter()
            .zip(lines.iter())
            .take_while(|(&(old_hash, _), line)| old_hash == hash_of(line))
            .count();
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

    fn splice_inserted_lines(&mut self, lines: &[String], prefix: usize, initial: Continuation) {
        let added = lines.len() - self.line_count;
        let state_at_prefix = self.state_before(prefix).unwrap_or(initial);
        self.spans
            .splice(prefix..prefix, std::iter::repeat_with(Vec::new).take(added));
        self.states.splice(
            prefix..prefix,
            lines
                .get(prefix..prefix + added)
                .into_iter()
                .flatten()
                .map(|line| (!hash_of(line), state_at_prefix)),
        );
    }

    fn drain_removed_lines(&mut self, lines: &[String], prefix: usize, initial: Continuation) {
        let removed = (self.line_count - lines.len()).min(self.states.len() - prefix);
        let old_in = self.state_before(prefix + removed).unwrap_or(initial);
        let new_in = self.state_before(prefix).unwrap_or(initial);
        self.spans.drain(prefix..prefix + removed);
        self.states.drain(prefix..prefix + removed);
        if let Some((slot, line)) = self
            .states
            .get_mut(prefix)
            .zip(lines.get(prefix))
            .filter(|_| old_in != new_in)
        {
            slot.0 = !hash_of(line);
        }
    }

    fn reconcile_in_place(&mut self, lines: &[String], end: usize, initial: Continuation) {
        let limit = self.spans.len().min(lines.len());
        let mut incoming = initial;
        let mut old_incoming = initial;
        for (idx, ((line, state_slot), span_slot)) in lines
            .iter()
            .zip(self.states.iter_mut())
            .zip(self.spans.iter_mut())
            .take(limit)
            .enumerate()
        {
            let line_hash = hash_of(line);
            let (old_hash, old_after) = *state_slot;
            if incoming == old_incoming && line_hash == old_hash {
                incoming = old_after;
                old_incoming = old_after;
                continue;
            }
            if idx >= end {
                self.spans.truncate(idx);
                self.states.truncate(idx);
                return;
            }
            self.engine.resume(incoming);
            *span_slot = lex_line(&mut self.engine, line);
            let new_after = self.engine.continuation().unwrap_or(incoming);
            *state_slot = (line_hash, new_after);
            incoming = new_after;
            old_incoming = old_after;
        }
    }

    fn state_before(&self, idx: usize) -> Option<Continuation> {
        let init = self.initial?;
        idx.checked_sub(1)
            .map_or(Some(init), |prev| self.states.get(prev).map(|&(_, s)| s))
    }

    pub(super) fn extend(&mut self, lines: &[String], end: usize) {
        if !self.engine.is_active() {
            return;
        }
        while self.spans.len() < end {
            let idx = self.spans.len();
            let Some(line) = lines.get(idx) else {
                break;
            };
            if let Some(prev) = self.state_before(idx) {
                self.engine.resume(prev);
            }
            let spans = lex_line(&mut self.engine, line);
            if let Some(after) = self.engine.continuation() {
                self.states.push((hash_of(line), after));
            }
            self.spans.push(spans);
        }
    }
}
