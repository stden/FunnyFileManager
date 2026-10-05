//! Reuse lexical spans across cursor-only frames; source equality prevents stale colours.
use super::{Highlighter, Token};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct HighlightCache(Option<Box<Document>>);

struct Document {
    path: PathBuf,
    source: Vec<String>,
    engine: Highlighter,
    spans: Vec<Vec<(usize, usize, Token)>>,
}

// Cloning editor content starts a fresh cache; cached presentation is not document state.
impl Clone for HighlightCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl HighlightCache {
    pub(crate) fn prepare(&mut self, path: &Path, lines: &[String], end: usize) {
        let valid = self
            .0
            .as_ref()
            .is_some_and(|document| document.path == path && document.source == lines);
        if !valid {
            self.0 = Some(Box::new(Document {
                path: path.to_path_buf(),
                source: lines.to_vec(),
                engine: Highlighter::for_file(path, lines),
                spans: Vec::new(),
            }));
        }
        if let Some(document) = &mut self.0 {
            document.extend(end);
        }
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

impl Document {
    fn extend(&mut self, end: usize) {
        if !self.engine.is_active() {
            return;
        }
        while self.spans.len() < end {
            let Some(line) = self.source.get(self.spans.len()) else {
                break;
            };
            let mut start = 0;
            let spans = self
                .engine
                .line(line)
                .into_iter()
                .map(|(text, token)| {
                    let end = start + text.len();
                    let span = (start, end, token);
                    start = end;
                    span
                })
                .collect();
            self.spans.push(spans);
        }
    }
}

#[cfg(test)]
mod tests;
