//! Purpose: A live completion refresh with the cursor off the text leaves the
//! popup as it was, because there is no position to refresh it at.

use super::*;

#[test]
fn a_live_refresh_with_the_cursor_off_the_text_leaves_the_popup_alone() {
    let root = TempDir::new().unwrap();
    let mut ed = editor(&root, "get\n");
    ed.lsp.state = ServerState::Ready;
    ed.cursor_col = 3;
    let first = query(&ed);
    ed.update_live_completion(first, vec![completion(json!({"label": "get_value"}))]);
    assert!(matches!(ed.lsp.popup, Some(Dialog::Completion { .. })));

    let stale = query(&ed);
    ed.cursor_line = 9;
    ed.update_live_completion(stale, Vec::new());
    assert!(
        matches!(ed.lsp.popup, Some(Dialog::Completion { .. })),
        "the popup stays as it was"
    );
}
