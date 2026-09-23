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
                SyncKind::Full | SyncKind::None => json!({"text":snapshot.text}),
            };
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

fn incremental(old: &str, new: &str) -> Value {
    let prefix: usize = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum();
    let suffix: usize = old[prefix..]
        .chars()
        .rev()
        .zip(new[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum();
    // Prefix/suffix are sums of complete UTF-8 characters, hence valid boundaries.
    let start = positions::at_byte(old, prefix).expect("prefix is a UTF-8 boundary");
    let end = positions::at_byte(old, old.len() - suffix).expect("suffix is a UTF-8 boundary");
    json!({"range":{"start":start,"end":end}, "text":new[prefix..new.len()-suffix]})
}
