//! Reuse lexical spans across cursor-only frames and converge in O(1) across single-line edits.
use super::{highlighter::Continuation, Highlighter, Token};
use std::path::{Path, PathBuf};

mod document;
use document::LineState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) token: Token,
}

type LineSpans = Vec<Span>;

#[derive(Debug, Default)]
pub struct HighlightCache(Option<Box<Document>>);

#[derive(Debug)]
struct Document {
    path: PathBuf,
    #[cfg(test)]
    fingerprint: u64,
    revision: Option<u64>,
    line_count: usize,
    engine: Highlighter,
    initial: Option<Continuation>,
    states: Vec<LineState>,
    spans: Vec<Option<LineSpans>>,
}

// Cloning editor content starts a fresh cache; cached presentation is not document state.
impl Clone for HighlightCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for HighlightCache {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}
impl Eq for HighlightCache {}

const FX_HASH_SEED: u64 = 0x517c_c1b7_2722_0a95;

#[inline]
const fn fx_mix(state: u64, word: u64) -> u64 {
    (state ^ word).rotate_left(5).wrapping_mul(FX_HASH_SEED)
}

fn hash_of(line: &str) -> u64 {
    #[cfg(test)]
    crate::highlight::HASH_PASSES.set(crate::highlight::HASH_PASSES.get() + 1);
    let bytes = line.as_bytes();
    let mut hash = u64::try_from(bytes.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(FX_HASH_SEED);
    let (chunks, rem) = bytes.as_chunks::<8>();
    for &word in chunks {
        hash = fx_mix(hash, u64::from_le_bytes(word));
    }
    if !rem.is_empty() {
        let mut tail = [0_u8; 8];
        if let Some(dst) = tail.get_mut(..rem.len()) {
            dst.copy_from_slice(rem);
        }
        hash = fx_mix(hash, u64::from_le_bytes(tail));
    }
    hash ^ (hash >> 32)
}

#[cfg(test)]
fn fingerprint_of(lines: &[String]) -> u64 {
    let seed = u64::try_from(lines.len()).unwrap_or(0);
    lines
        .iter()
        .fold(seed, |acc, line| acc.rotate_left(7) ^ hash_of(line))
}

impl HighlightCache {
    #[cfg(test)]
    pub(crate) fn prepare(&mut self, path: &Path, lines: &[String], end: usize) {
        let end = end.min(lines.len());
        let fingerprint = fingerprint_of(lines);
        if let Some(doc) = self.0.as_mut().filter(|doc| doc.path == path) {
            if doc.fingerprint == fingerprint && doc.line_count == lines.len() {
                doc.ensure_window(lines, 0, end);
                return;
            }
            if let Some(initial) = doc.initial {
                doc.fingerprint = fingerprint;
                doc.revision = None;
                doc.reconcile(lines, 0, end, initial, None);
                return;
            }
        }
        let mut doc = Document::new(path, lines, fingerprint, None);
        doc.ensure_window(lines, 0, end);
        self.0 = Some(Box::new(doc));
    }

    #[cfg(test)]
    pub(crate) fn prepare_revision(
        &mut self,
        path: &Path,
        lines: &[String],
        end: usize,
        revision: u64,
    ) {
        self.prepare_window(path, lines, 0, end, revision, None);
    }

    /// `dirty` is `(first changed line, first unchanged line)` when the editor
    /// knows it. `None` hashes from line 0, which stays correct if `lines`
    /// was edited directly.
    #[cfg(test)]
    pub(crate) fn prepare_hinted(
        &mut self,
        path: &Path,
        lines: &[String],
        end: usize,
        revision: u64,
        dirty: Option<(usize, usize)>,
    ) {
        self.prepare_window(path, lines, 0, end, revision, dirty);
    }

    /// Prepare token spans for visible lines `top..end`, fast-forwarding
    /// continuation state over `0..top` without allocating off-screen token vectors.
    pub(crate) fn prepare_window(
        &mut self,
        path: &Path,
        lines: &[String],
        top: usize,
        end: usize,
        revision: u64,
        dirty: Option<(usize, usize)>,
    ) {
        let end = end.min(lines.len());
        let top = top.min(end);
        if let Some(doc) = self.0.as_mut().filter(|doc| doc.path == path) {
            if doc.revision == Some(revision) && doc.line_count == lines.len() {
                doc.ensure_window(lines, top, end);
                return;
            }
            if let Some(initial) = doc.initial {
                doc.revision = Some(revision);
                doc.reconcile(lines, top, end, initial, dirty);
                return;
            }
        }
        let mut doc = Document::new(path, lines, 0, Some(revision));
        doc.ensure_window(lines, top, end);
        self.0 = Some(Box::new(doc));
    }

    pub(crate) fn span_slice(&self, index: usize) -> Option<&[Span]> {
        self.0
            .as_ref()
            .and_then(|document| document.spans.get(index))
            .and_then(Option::as_deref)
    }

    pub(crate) fn splice_lines(&mut self, start: usize, old_end: usize, added: &[String]) {
        if let Some(doc) = self.0.as_mut() {
            doc.splice_lines(start, old_end, added);
        }
    }

    #[cfg(test)]
    pub(crate) fn runs<'a>(&self, index: usize, line: &'a str) -> Vec<(&'a str, Token)> {
        let Some(spans) = self.span_slice(index) else {
            return vec![(line, Token::Normal)];
        };
        spans
            .iter()
            .map(|&Span { start, end, token }| line.get(start..end).map(|text| (text, token)))
            .collect::<Option<_>>()
            .unwrap_or_else(|| vec![(line, Token::Normal)])
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_hint;
