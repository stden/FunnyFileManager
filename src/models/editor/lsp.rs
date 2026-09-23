//! Purpose: Editor-owned LSP context and transient results; contains no process or I/O state.

use super::{EditState, Editor};
pub use crate::lsp::types::{Feature as LspFeature, ServerState};
use crate::lsp::{
    features::{Completion, Definition, Diagnostic, Reply},
    positions::{self, Position},
    types::{DocumentId, Query, Revision, Snapshot, Stamp},
};

mod completion;

#[derive(Clone, Default)]
pub struct EditorLspState {
    pub(crate) id: DocumentId,
    pub(crate) revision: Revision,
    pub(crate) saved: u64,
    pub(crate) state: ServerState,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) intent: Intent,
    pub(crate) popup: Option<Dialog>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Intent {
    #[default]
    None,
    Query(LspFeature),
    Restart,
}

#[derive(Clone)]
pub enum Dialog {
    Completion {
        query: Query,
        items: Vec<Completion>,
        selected: usize,
    },
    Hover {
        text: String,
        scroll: u16,
    },
    Definitions {
        items: Vec<Definition>,
        selected: usize,
    },
}

impl Editor {
    pub(crate) fn mark_modified(&mut self) {
        self.state = EditState::Modified;
        self.lsp.revision.0 = self.lsp.revision.0.saturating_add(1);
        self.lsp.popup = None;
        self.lsp.diagnostics.clear();
    }

    pub(crate) const fn lsp_stamp(&self) -> Stamp {
        Stamp {
            id: self.lsp.id,
            revision: self.lsp.revision,
        }
    }

    pub(crate) fn lsp_position(&self) -> Option<Position> {
        positions::from_chars(
            self.cursor_line,
            self.lines.get(self.cursor_line)?,
            self.cursor_col,
        )
    }

    pub(crate) fn lsp_text(&self) -> String {
        let mut text = self.lines.join("\n");
        if self.format.trailing_newline {
            text.push('\n');
        }
        text
    }

    pub(crate) fn lsp_snapshot(&self) -> Snapshot {
        Snapshot {
            stamp: self.lsp_stamp(),
            path: self.path.clone(),
            text: self.lsp_text(),
            saved: self.lsp.saved,
        }
    }

    pub(crate) fn accepts_lsp_reply(&self, query: Query) -> bool {
        self.lsp_stamp() == query.stamp
            && self.lsp_position() == Some(query.position)
            && self.accepts_text()
    }

    pub(crate) fn show_lsp_reply(&mut self, query: Query, reply: Reply) -> Option<Definition> {
        if !self.accepts_lsp_reply(query) {
            return None;
        }
        match reply {
            Reply::Completions(items) if !items.is_empty() => {
                self.lsp.popup = Some(Dialog::Completion {
                    query,
                    items,
                    selected: 0,
                });
            }
            Reply::Hover(text) if !text.is_empty() => {
                self.lsp.popup = Some(Dialog::Hover { text, scroll: 0 });
            }
            Reply::Definitions(mut items) if items.len() == 1 => return items.pop(),
            Reply::Definitions(items) if !items.is_empty() => {
                self.lsp.popup = Some(Dialog::Definitions { items, selected: 0 });
            }
            _ => self.notice = Some("No language-server results".into()), // Empty results of all features.
        }
        None
    }
}

#[cfg(test)]
mod tests;
