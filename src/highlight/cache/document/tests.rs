//! Purpose: Unit tests for `Document::ensure_window` and tail line removal without `colorer`.

use super::*;

#[test]
fn ensure_window_exercises_both_initial_states_without_colorer() {
    let lines = vec!["/* open".to_owned(), "*/".to_owned()];
    let mut with_state = Document::new(Path::new("a.rs"), &lines, 0, Some(1));
    with_state.ensure_window(&lines, 0, 1);
    with_state.ensure_window(&lines, 0, 2);
    assert_eq!(with_state.states.len(), 2);

    let mut without_state = Document::new(Path::new("a.rs"), &lines, 0, Some(1));
    without_state.initial = None;
    without_state.ensure_window(&lines, 0, 1);
    assert!(without_state.states.is_empty());
    assert_eq!(without_state.spans.len(), 1);
}

#[test]
fn deleting_last_line_changing_continuation_skips_missing_trailing_slot() {
    let lines = vec!["/* open".to_owned(), "*/".to_owned()];
    let mut doc = Document::new(Path::new("b.rs"), &lines, 0, Some(1));
    doc.ensure_window(&lines, 0, 2);
    let init = doc.initial.expect("built-in scanner has continuation");
    let shorter = vec!["/* open".to_owned()];
    doc.reconcile(&shorter, 0, 1, init, None);
    assert_eq!(doc.states.len(), 1);
}
