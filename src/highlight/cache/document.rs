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
        fingerprint: u64,
        revision: Option<u64>,
    ) -> Self {
        let engine = Highlighter::for_file(path, lines);
        let initial = engine.continuation();
        Self {
            path: path.to_path_buf(),
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
            let prefix = self
                .states
                .iter()
                .zip(lines.iter())
                .take_while(|(&(old_hash, _), line)| old_hash == hash_of(line))
                .count();
            self.spans.truncate(prefix);
            self.states.truncate(prefix);
            self.line_count = lines.len();
            self.extend(lines, end);
            return;
        }
        self.reconcile_in_place(lines, end, initial);
        self.extend(lines, end);
    }

    fn reconcile_in_place(&mut self, lines: &[String], end: usize, initial: Continuation) {
        let limit = end.min(self.spans.len()).min(lines.len());
        let mut incoming = initial;
        let mut old_incoming = initial;
        for ((line, state_slot), span_slot) in lines
            .iter()
            .zip(self.states.iter_mut())
            .zip(self.spans.iter_mut())
            .take(limit)
        {
            let line_hash = hash_of(line);
            let (old_hash, old_after) = *state_slot;
            if incoming == old_incoming && line_hash == old_hash {
                incoming = old_after;
                old_incoming = old_after;
                continue;
            }
            self.engine.resume(incoming);
            *span_slot = lex_line(&mut self.engine, line);
            let new_after = self.engine.continuation().unwrap_or(incoming);
            *state_slot = (line_hash, new_after);
            incoming = new_after;
            old_incoming = old_after;
        }
        if incoming != old_incoming {
            self.spans.truncate(limit);
            self.states.truncate(limit);
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
