//! Purpose: Unit tests for Jira user display labels and comment author matching.

use super::JiraUser;

#[test]
fn display_label_falls_back_from_display_name_to_name_to_key() {
    let full = JiraUser {
        name: "alice".to_owned(),
        key: "k-alice".to_owned(),
        display_name: "Alice Example".to_owned(),
    };
    assert_eq!(full.display_label(), "Alice Example");

    let named = JiraUser {
        display_name: String::new(),
        ..full
    };
    assert_eq!(named.display_label(), "alice");

    let keyed = JiraUser {
        name: String::new(),
        ..named
    };
    assert_eq!(keyed.display_label(), "k-alice");
}

#[test]
fn matches_author_checks_both_name_and_key_without_empty_false_positives() {
    let key_only = JiraUser {
        name: String::new(),
        key: "user-42".to_owned(),
        display_name: String::new(),
    };
    assert!(key_only.matches_author("user-42", ""));
    assert!(key_only.matches_author("", "user-42"));
    assert!(!key_only.matches_author("", ""));
    assert!(!key_only.matches_author("other", "other"));
}
