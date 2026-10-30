//! Reuse lexical spans across cursor-only frames and converge in O(1) across single-line edits.
use super::{highlighter::Continuation, Highlighter, Token};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

mod document;

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
    states: Vec<(u64, Continuation)>,
    spans: Vec<Vec<(usize, usize, Token)>>,
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

fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

impl HighlightCache {
    #[cfg(test)]
    pub(crate) fn prepare(&mut self, path: &Path, lines: &[String], end: usize) {
        let end = end.min(lines.len());
        let fingerprint = hash_of(lines);
        if let Some(doc) = self.0.as_mut().filter(|doc| doc.path == path) {
            if doc.fingerprint == fingerprint && doc.line_count == lines.len() {
                doc.extend(lines, end);
                return;
            }
            if let Some(initial) = doc.initial {
                doc.fingerprint = fingerprint;
                doc.revision = None;
                doc.reconcile(lines, end, initial);
                return;
            }
        }
        let mut doc = Document::new(path, lines, fingerprint, None);
        doc.extend(lines, end);
        self.0 = Some(Box::new(doc));
    }

    pub(crate) fn prepare_revision(
        &mut self,
        path: &Path,
        lines: &[String],
        end: usize,
        revision: u64,
    ) {
        let end = end.min(lines.len());
        if let Some(doc) = self.0.as_mut().filter(|doc| doc.path == path) {
            if doc.revision == Some(revision) && doc.line_count == lines.len() {
                doc.extend(lines, end);
                return;
            }
            if let Some(initial) = doc.initial {
                doc.revision = Some(revision);
                doc.reconcile(lines, end, initial);
                return;
            }
        }
        let mut doc = Document::new(path, lines, 0, Some(revision));
        doc.extend(lines, end);
        self.0 = Some(Box::new(doc));
    }

    pub(crate) fn runs<'a>(&self, index: usize, line: &'a str) -> Vec<(&'a str, Token)> {
        let Some(spans) = self
            .0
            .as_ref()
            .and_then(|document| document.spans.get(index))
        else {
            return vec![(line, Token::Normal)];
        };
        // Spans are byte ranges of the line they were computed from; a line
        // they do not fit is drawn plain rather than cut mid-character.
        spans
            .iter()
            .map(|&(start, end, token)| line.get(start..end).map(|text| (text, token)))
            .collect::<Option<_>>()
            .unwrap_or_else(|| vec![(line, Token::Normal)])
    }
}

#[cfg(test)]
mod tests;
