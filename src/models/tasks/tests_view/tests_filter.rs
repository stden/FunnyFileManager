//! Purpose: unit tests for the filter's terms and the user scope.

use super::*;

fn filtered(snapshot: &Snapshot, line: &str) -> Vec<u32> {
    let options = ViewOptions {
        filter: FilterTerms::parse(line),
        sort: Sort::new(SortKey::Pid, SortOrder::Direct),
        ..ViewOptions::default()
    };
    pids(snapshot, &build_view(snapshot.rows(), &options))
}

#[test]
fn the_filter_matches_any_term_in_the_name_or_the_command_line() {
    let snapshot = browsers();
    assert_eq!(filtered(&snapshot, "fire|chr"), [1, 2]);
    assert_eq!(filtered(&snapshot, "train"), [3]);
    assert_eq!(filtered(&snapshot, "incognito"), [2]);
}

#[test]
fn the_filter_ignores_case_and_blank_terms() {
    let snapshot = browsers();
    assert_eq!(filtered(&snapshot, "  FIRE  ||"), [1]);
    let blank = FilterTerms::parse(" | |");
    assert!(blank.is_empty());
    assert_eq!(blank.line(), " | |");
    assert_eq!(filtered(&snapshot, " | |"), [1, 2, 3]);
    assert_eq!(filtered(&snapshot, ""), [1, 2, 3]);
}

#[test]
fn a_user_scope_shows_only_that_owners_processes() {
    let alice = ProcessSample {
        owner: Owner::Named("alice".to_owned()),
        ..process(2, "editor")
    };
    let snapshot = snapshot(vec![process(1, "shell"), alice]);
    let only_bob = ViewOptions {
        scope: UserScope::Only(Owner::Named("bob".to_owned())),
        ..ViewOptions::default()
    };
    assert_eq!(
        pids(&snapshot, &build_view(snapshot.rows(), &only_bob)),
        [1]
    );
    let everyone = build_view(snapshot.rows(), &ViewOptions::default());
    assert_eq!(everyone.len(), 2);
}
