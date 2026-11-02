use super::*;

#[test]
fn case_insensitive_search_handles_empty_patterns_and_unicode() {
    assert!(contains_lowercased("anything", ""));
    assert!(contains_lowercased("Straße ÄPFEL", "äpfel"));
    assert!(!contains_lowercased("Straße ÄPFEL", "birne"));
}

#[test]
fn inaccessible_or_invalid_text_does_not_turn_into_a_match() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("absent");
    let invalid = root.path().join("invalid.txt");
    fs::write(&invalid, [0xff, 0xfe]).unwrap();
    let mut results = Vec::new();
    grep_file(&missing, "", &mut results, 10);
    grep_file(&invalid, "", &mut results, 10);
    grep_recursive(&missing, "", None, &mut results, 10, &mut 0);
    assert!(results.is_empty());
}

#[test]
fn content_walk_stops_before_opening_entries_when_its_budget_or_limit_is_full() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("one.txt"), b"match").unwrap();
    let mut results = Vec::new();
    let mut visited = crate::walk::WALK_BUDGET;
    grep_recursive(root.path(), "match", None, &mut results, 10, &mut visited);
    grep_recursive(root.path(), "match", None, &mut results, 0, &mut 0);
    assert!(results.is_empty());
    assert_eq!(visited, crate::walk::WALK_BUDGET);
}
