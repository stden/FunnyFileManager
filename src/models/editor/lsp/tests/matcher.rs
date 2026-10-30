//! Purpose: Unit tests for `IntelliJ`-style completion matching, ranking, and live filtering.

use super::super::matcher::filter_and_rank;
use super::*;

#[test]
fn camel_humps_and_prefix_matcher_ranks_candidates_like_intellij() {
    let items: Vec<_> = [
        "other",
        "has_GMV_case",
        "has_gmv_in",
        "getMyVariable",
        "get_my_value",
        "GMVUpper",
        "gmvExact",
        "gmv",
    ]
    .into_iter()
    .map(|label| completion(json!({"label": label})))
    .collect();

    assert_eq!(filter_and_rank(&items, "").len(), items.len());

    let ranked = filter_and_rank(&items, "gmv");
    let labels: Vec<&str> = ranked.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "gmv",
            "gmvExact",
            "GMVUpper",
            "getMyVariable",
            "get_my_value",
            "has_gmv_in",
            "has_GMV_case",
        ]
    );
}

#[test]
fn matcher_falls_back_to_insert_text_and_handles_non_identifier_stems() {
    let by_insert = completion(json!({"label": "display only", "insertText": "target_fn()"}));
    let symbol_only = completion(json!({"label": "...", "insertText": ""}));
    let items = vec![symbol_only, by_insert];
    let ranked = filter_and_rank(&items, "tf");
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].label, "display only");
}

#[test]
fn live_completion_updates_text_edit_range_and_replaces_identifier_suffix() {
    assert!(is_identifier_char('_'));
    let root = TempDir::new().unwrap();
    let mut ed = editor(&root, "get_old\n");
    ed.lsp.state = ServerState::Ready;
    ed.cursor_col = 3;
    let old_q = query(&ed);
    let item = completion(json!({
        "label": "get_value",
        "textEdit": {
            "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 3}},
            "newText": "get_value"
        }
    }));
    ed.insert_char('_');
    assert_eq!(ed.completion_prefix(), "get_");
    ed.update_live_completion(old_q, vec![item]);
    let Some(Dialog::Completion {
        query: new_q,
        items,
        ..
    }) = ed.lsp.popup.clone()
    else {
        panic!("expected live completion popup");
    };
    assert_eq!(items[0].edit.as_ref().unwrap().range.end, new_q.position);
    ed.complete_lsp_replace(new_q, &items[0]).unwrap();
    assert_eq!(ed.lines[0], "get_value");
}

#[test]
fn live_completion_clears_pool_when_nothing_matches() {
    let root = TempDir::new().unwrap();
    let mut ed = editor(&root, "get\n");
    ed.cursor_col = 3;
    let q = query(&ed);
    ed.insert_char('z');
    ed.update_live_completion(q, vec![completion(json!({"label": "get_value"}))]);
    assert!(ed.lsp.popup.is_none());
    assert!(ed.lsp.completions.is_none());
}

#[test]
fn silent_completion_skips_unmatched_popup_and_triggers_follow_operators() {
    let root = TempDir::new().unwrap();
    let mut ed = editor(&root, "xyz");
    ed.lsp.state = ServerState::Ready;
    ed.cursor_col = 3;
    let q = query(&ed);
    let items = vec![completion(json!({"label": "abc"}))];

    // Silent auto-popup does not open when nothing matches the current prefix.
    ed.open_completion_popup(q, items.clone(), CompletionTrigger::AutoWhileTyping);
    assert!(ed.lsp.popup.is_none());

    // Explicit Ctrl+Space (`CompletionTrigger::Manual`) shows the full fallback list even if unmatched.
    ed.open_completion_popup(q, items, CompletionTrigger::Manual);
    assert!(matches!(ed.lsp.popup, Some(Dialog::Completion { .. })));

    // Trigger checks: `.`, `::`, `->`, identifier char vs `;`.
    ed.lines = vec!["foo.".into()];
    ed.cursor_col = 4;
    ed.maybe_trigger_completion('.');
    assert!(ed.lsp.auto_complete);

    ed.lines = vec!["std::".into()];
    ed.cursor_col = 5;
    ed.maybe_trigger_completion(':');
    assert!(ed.lsp.auto_complete);

    ed.lines = vec!["ptr->".into()];
    ed.cursor_col = 5;
    ed.maybe_trigger_completion('>');
    assert!(ed.lsp.auto_complete);

    ed.lines = vec!["x;".into()];
    ed.cursor_col = 2;
    ed.maybe_trigger_completion(';');
    assert!(!ed.lsp.auto_complete);
}
