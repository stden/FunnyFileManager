//! Unit tests: stamp-validated ignore cache — lookup, store, and invalidation.

use super::{invalidate_under, lookup, store, IgnoreStamp};
use tempfile::TempDir;

fn zero_stamp() -> IgnoreStamp {
    IgnoreStamp {
        dir_stamp: None,
        index_stamp: None,
        rules_hash: 0,
        names_hash: 0,
    }
}

fn stamp(rules: u64, names: u64) -> IgnoreStamp {
    IgnoreStamp {
        dir_stamp: None,
        index_stamp: None,
        rules_hash: rules,
        names_hash: names,
    }
}

#[test]
fn lookup_returns_none_for_unknown_directory() {
    let dir = TempDir::new().expect("temp");
    assert!(lookup(dir.path(), zero_stamp()).is_none());
}

#[test]
fn store_then_lookup_with_matching_stamp_returns_the_list() {
    let dir = TempDir::new().expect("temp");
    let s = stamp(1, 2);
    let ignored = vec!["target".to_owned(), ".cache".to_owned()];
    store(dir.path(), s, &ignored);
    assert_eq!(lookup(dir.path(), s), Some(ignored));
}

#[test]
fn lookup_returns_none_when_stamp_differs() {
    let dir = TempDir::new().expect("temp");
    store(dir.path(), stamp(10, 20), &["a".to_owned()]);
    // Different rules_hash — cache miss.
    assert!(lookup(dir.path(), stamp(99, 20)).is_none());
}

#[test]
fn invalidate_under_clears_subdirectory_entries() {
    let repo = TempDir::new().expect("temp");
    let sub = repo.path().join("sub");
    let s = stamp(3, 4);
    store(&sub, s, &["out".to_owned()]);
    assert!(
        lookup(&sub, s).is_some(),
        "must be cached before invalidation"
    );
    invalidate_under(repo.path());
    assert!(
        lookup(&sub, s).is_none(),
        "must be evicted after invalidation"
    );
}

#[test]
fn invalidate_under_keeps_unrelated_entries() {
    let repo_a = TempDir::new().expect("temp");
    let repo_b = TempDir::new().expect("temp");
    let s = stamp(5, 6);
    store(repo_a.path(), s, &["dist".to_owned()]);
    store(repo_b.path(), s, &["out".to_owned()]);
    invalidate_under(repo_a.path());
    // repo_b is unrelated; it should still be cached.
    assert!(lookup(repo_b.path(), s).is_some());
}
