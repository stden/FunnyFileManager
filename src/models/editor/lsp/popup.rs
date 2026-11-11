//! Purpose: Open, filter, and auto-trigger the editor's live LSP completion popup.

use super::{
    matcher::filter_and_rank, CompletionPool, CompletionTrigger, Dialog, Editor, ServerState,
};
use crate::lsp::{features::Completion, positions::Position, types::Query};

impl Editor {
    pub(super) fn open_completion_popup(
        &mut self,
        query: Query,
        items: Vec<Completion>,
        trigger: CompletionTrigger,
    ) {
        self.lsp.completions = Some(Box::new(CompletionPool(items.clone())));
        let filtered = filter_and_rank(&items, self.completion_prefix_str());
        if filtered.is_empty() && trigger == CompletionTrigger::AutoWhileTyping {
            self.lsp.popup = None;
            return;
        }
        let shown = if filtered.is_empty() { items } else { filtered };
        self.lsp.popup = Some(Dialog::Completion {
            query,
            items: shown,
            selected: 0,
        });
    }

    pub(crate) fn update_live_completion(&mut self, old_query: Query, fallback: Vec<Completion>) {
        let Some(position) = self.lsp_position() else {
            return;
        };
        let mut pool = self
            .lsp
            .completions
            .take()
            .unwrap_or_else(|| Box::new(CompletionPool(fallback)));
        extend_edit_ends(&mut pool.0, old_query.position, position);
        let items = filter_and_rank(&pool.0, self.completion_prefix_str());
        if items.is_empty() {
            self.lsp.popup = None;
            return;
        }
        self.lsp.completions = Some(pool);
        self.lsp.popup = Some(Dialog::Completion {
            query: Query {
                stamp: self.lsp_stamp(),
                position,
                feature: old_query.feature,
            },
            items,
            selected: 0,
        });
    }

    pub(crate) fn maybe_trigger_completion(&mut self, ch: char) {
        if self.lsp.state == ServerState::Ready {
            let before = self
                .line()
                .get(..self.byte_col(self.cursor_col))
                .unwrap_or_default();
            let prev = before.strip_suffix(ch).and_then(|s| s.chars().next_back());
            self.lsp.auto_complete = super::is_identifier_char(ch)
                || ch == '.'
                || matches!((prev, ch), (Some(':'), ':') | (Some('-'), '>'));
        }
    }
}

fn extend_edit_ends(items: &mut [Completion], old_end: Position, new_end: Position) {
    for edit in items
        .iter_mut()
        .filter_map(|item| item.edit.as_mut())
        .filter(|edit| edit.range.end == old_end)
    {
        edit.range.end = new_end;
    }
}
