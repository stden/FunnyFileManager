//! Purpose: Versioned document notifications and UTF-16 incremental text replacements.

use super::{positions, rpc::Session, types::Snapshot};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SyncKind {
    None,
    Full,
    Incremental,
}

pub(super) struct Document {
    pub snapshot: Snapshot,
    pub uri: String,
    pub version: i32,
}

impl Document {
    pub fn open(snapshot: Snapshot, uri: String, language: &str, rpc: &mut Session) -> Self {
        rpc.notify(
            "textDocument/didOpen",
            json!({"textDocument": {
                "uri":uri, "languageId":language, "version":1, "text":snapshot.text
            }}),
        );
        Self {
            snapshot,
            uri,
            version: 1,
        }
    }

    pub fn update(
        &mut self,
        snapshot: Snapshot,
        kind: SyncKind,
        rpc: &mut Session,
    ) -> Result<(), &'static str> {
        if self.snapshot.text != snapshot.text && kind != SyncKind::None {
            self.version = self
                .version
                .checked_add(1)
                .ok_or("Document version overflow")?;
            let change = match kind {
                SyncKind::Incremental => incremental(&self.snapshot.text, &snapshot.text),
                SyncKind::Full | SyncKind::None => None,
            }
            .unwrap_or_else(|| json!({"text":snapshot.text}));
            rpc.notify("textDocument/didChange", json!({
                "textDocument":{"uri":self.uri, "version":self.version}, "contentChanges":[change]
            }));
        }
        self.snapshot = snapshot;
        Ok(())
    }

    pub fn close(&self, rpc: &mut Session) {
        rpc.notify(
            "textDocument/didClose",
            json!({"textDocument":{"uri":self.uri}}),
        );
    }
}

/// The edit from `old` to `new` as one replaced range. `None` only if a split
/// point fell inside a character, and the caller then sends the whole text.
fn incremental(old: &str, new: &str) -> Option<Value> {
    let mut prefix = crate::complete::common_prefix_len(old, new);
    if inside_crlf(old, prefix) || inside_crlf(new, prefix) {
        prefix -= 1;
    }
    let (_, old_rest) = old.split_at_checked(prefix)?;
    let (_, new_rest) = new.split_at_checked(prefix)?;
    let mut suffix: usize = old_rest
        .chars()
        .rev()
        .zip(new_rest.chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum();
    if inside_crlf(old, old.len() - suffix) || inside_crlf(new, new.len() - suffix) {
        suffix -= 1;
    }
    // Complete UTF-8 characters and complete line endings define LSP positions.
    let start = positions::at_byte(old, prefix)?;
    let end = positions::at_byte(old, old.len() - suffix)?;
    let text = new.get(prefix..new.len() - suffix)?;
    Some(json!({"range":{"start":start,"end":end}, "text":text}))
}

fn inside_crlf(text: &str, offset: usize) -> bool {
    text.split_at_checked(offset)
        .is_some_and(|(before, after)| before.ends_with('\r') && after.starts_with('\n'))
}
