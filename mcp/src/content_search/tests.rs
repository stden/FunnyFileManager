use super::*;

#[test]
fn case_insensitive_search_handles_empty_patterns_and_unicode() {
    assert!(contains_lowercased("anything", ""));
    assert!(contains_lowercased("service-0007_cfg.toml", "0007_cfg"));
    assert!(!contains_lowercased("short", "longer_needle"));
    assert!(contains_lowercased("Straße ÄPFEL", "äpfel"));
    assert!(!contains_lowercased("Straße ÄPFEL", "birne"));
    assert!(contains_lowercased("Отчёт_README.md", "readme"));
    assert!(!contains_lowercased("Отчёт_данные.md", "readme"));
    assert!(contains_lowercased("İstanbul", "i"));
    assert!(contains_lowercased("\u{212a}elvin", "kelvin"));
}

#[test]
fn inaccessible_or_invalid_text_does_not_turn_into_a_match() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("absent");
    let invalid = root.path().join("invalid.txt");
    fs::write(&invalid, [0xff, 0xfe]).unwrap();
    let mut results = Vec::new();
    let mut scratch = String::new();
    grep_file(&missing, "", &mut scratch, &mut results, 10);
    grep_file(&invalid, "", &mut scratch, &mut results, 10);
    grep_recursive(&missing, "", None, &mut scratch, &mut results, 10, &mut 0);
    assert_eq!(results, Vec::<serde_json::Value>::new());
}

#[test]
fn content_walk_stops_before_opening_entries_when_its_budget_or_limit_is_full() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("one.txt"), b"match").unwrap();
    let mut results = Vec::new();
    let mut scratch = String::new();
    let mut visited = crate::walk::WALK_BUDGET;
    grep_recursive(
        root.path(),
        "match",
        None,
        &mut scratch,
        &mut results,
        10,
        &mut visited,
    );
    grep_recursive(
        root.path(),
        "match",
        None,
        &mut scratch,
        &mut results,
        0,
        &mut 0,
    );
    assert_eq!(results, Vec::<serde_json::Value>::new());
    assert_eq!(visited, crate::walk::WALK_BUDGET);
}

#[test]
fn content_walk_skips_directory_links_across_platforms() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.txt"), b"needle").unwrap();
    crate::tests_tools::make_dir_link_cross(outside.path(), &root.path().join("linked"));

    let mut results = Vec::new();
    let mut scratch = String::new();
    grep_recursive(
        root.path(),
        "needle",
        None,
        &mut scratch,
        &mut results,
        10,
        &mut 0,
    );
    assert_eq!(results, Vec::<serde_json::Value>::new());
}
