//! Purpose: A reply that does not answer the request it came for changes nothing.

use super::*;

#[test]
fn an_empty_hover_answering_an_automatic_completion_changes_nothing() {
    let root = TempDir::new().unwrap();
    let mut ed = editor(&root, "get\n");
    ed.lsp.state = ServerState::Ready;
    ed.cursor_col = 3;
    let asked = Query {
        stamp: ed.lsp_stamp(),
        position: ed.lsp_position().unwrap(),
        feature: LspFeature::Completion(CompletionTrigger::AutoWhileTyping),
    };
    assert_eq!(ed.show_lsp_reply(asked, Reply::Hover(String::new())), None);
    assert!(ed.lsp.popup.is_none());
    assert_eq!(ed.notice, None);
}
