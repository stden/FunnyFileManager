//! Purpose: Document versions and negotiated synchronization against captured messages.

use super::super::{document::*, positions, rpc::Session, types::*};
use serde_json::json;

pub(super) fn snapshot(text: &str) -> Snapshot {
    Snapshot {
        stamp: Stamp {
            id: DocumentId::default(),
            revision: Revision::default(),
        },
        path: std::path::PathBuf::from("test.rs"),
        text: text.into(),
        saved: 0,
    }
}

#[test]
fn opening_and_closing_use_document_identifiers_without_source_on_close() {
    let mut rpc = Session::default();
    let document = Document::open(
        snapshot("let x = 1;"),
        "file:///test.rs".into(),
        "rust",
        &mut rpc,
    );
    let open = &rpc.outbox[0];
    assert_eq!(open["method"], "textDocument/didOpen");
    assert_eq!(
        open["params"]["textDocument"],
        json!({"uri":"file:///test.rs", "languageId":"rust", "version":1,"text":"let x = 1;"})
    );
    document.close(&mut rpc);
    assert_eq!(
        rpc.outbox[1]["params"],
        json!({"textDocument":{"uri":"file:///test.rs"}})
    );
}

#[test]
fn full_sync_increments_versions_only_when_text_changes() {
    let mut rpc = Session::default();
    let mut doc = Document::open(snapshot("old"), "file:///test.rs".into(), "rust", &mut rpc);
    doc.update(snapshot("new"), SyncKind::Full, &mut rpc)
        .unwrap();
    assert_eq!(rpc.outbox[1]["params"]["textDocument"]["version"], 2);
    assert_eq!(
        rpc.outbox[1]["params"]["contentChanges"],
        json!([{"text":"new"}])
    );
    doc.update(snapshot("new"), SyncKind::Full, &mut rpc)
        .unwrap();
    assert_eq!(rpc.outbox.len(), 2);
    doc.update(snapshot("latest"), SyncKind::Full, &mut rpc)
        .unwrap();
    assert_eq!(doc.version, 3);
}

#[test]
fn incremental_changes_reconstruct_unicode_insertions_deletions_and_replacements() {
    for (old, new) in [
        ("a😀б\nrest", "a🚀б\nrest"),
        ("abc", "ac"),
        ("", "Я😀\nx"),
        ("a\nb", ""),
        ("abc", "abc!"),
    ] {
        let mut rpc = Session::default();
        let mut doc = Document::open(snapshot(old), "file:///test.rs".into(), "rust", &mut rpc);
        doc.update(snapshot(new), SyncKind::Incremental, &mut rpc)
            .unwrap();
        let change = &rpc.outbox[1]["params"]["contentChanges"][0];
        let range = serde_json::from_value(change["range"].clone()).unwrap();
        let mut reconstructed = old.to_owned();
        reconstructed.replace_range(
            positions::byte_range(old, range).unwrap(),
            change["text"].as_str().unwrap(),
        );
        assert_eq!(reconstructed, new);
    }
}

#[test]
fn sync_none_does_not_send_changes_and_version_exhaustion_is_an_error() {
    let mut rpc = Session::default();
    let mut doc = Document::open(snapshot("old"), "file:///test.rs".into(), "rust", &mut rpc);
    doc.update(snapshot("new"), SyncKind::None, &mut rpc)
        .unwrap();
    assert_eq!(rpc.outbox.len(), 1);
    doc.version = i32::MAX;
    assert!(doc
        .update(snapshot("changed"), SyncKind::Full, &mut rpc)
        .is_err());
}
