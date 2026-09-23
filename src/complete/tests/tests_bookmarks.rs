//! Purpose: Bookmark slot completion (populated slots and path hints).

use super::*;

// ── bookmark slot completion (populated slots + path hints) ────────

fn ctx_with_bookmarks<'a>(cwd: &'a Path, slots: &'a [Option<PathBuf>]) -> CompletionContext<'a> {
    CompletionContext {
        cwd,
        bookmarks: slots,
    }
}

#[test]
fn bookmark_go_offers_only_populated_slots_with_path_hints() {
    let tmp = empty_cwd();
    let mut slots = vec![None; 10];
    slots[0] = Some(PathBuf::from("/srv/app"));
    slots[3] = Some(PathBuf::from("/home/me/dl"));

    let c = complete_with("bookmark go ", &ctx_with_bookmarks(tmp.path(), &slots))
        .expect("populated slots complete");
    assert_eq!(texts(&c), vec!["0".to_string(), "3".to_string()]);
    // The path rides along as the dropdown hint.
    let hint0 = c
        .candidates
        .iter()
        .find(|c| c.text == "0")
        .unwrap()
        .hint
        .as_deref();
    let hint3 = c
        .candidates
        .iter()
        .find(|c| c.text == "3")
        .unwrap()
        .hint
        .as_deref();
    assert_eq!(hint0, Some("/srv/app"));
    assert_eq!(hint3, Some("/home/me/dl"));
}

#[test]
fn bookmark_go_with_no_populated_slots_completes_nothing() {
    let tmp = empty_cwd();
    let slots = vec![None; 10];
    assert!(
        complete_with("bookmark go ", &ctx_with_bookmarks(tmp.path(), &slots)).is_none(),
        "no populated slot means nothing to offer"
    );
}

#[test]
fn bookmark_go_filters_populated_slots_by_typed_digit() {
    let tmp = empty_cwd();
    let mut slots = vec![None; 10];
    slots[3] = Some(PathBuf::from("/a"));
    slots[7] = Some(PathBuf::from("/b"));

    let c = complete_with("bookmark go 3", &ctx_with_bookmarks(tmp.path(), &slots))
        .expect("slot 3 matches");
    assert_eq!(texts(&c), vec!["3".to_string()]);
    assert_eq!(c.line, "bookmark go 3 ");
}

#[test]
fn bookmark_go_without_context_bookmarks_offers_nothing() {
    // The bare `complete()` wrapper has no bookmark slots, so `go` cannot list
    // any — it must not fall back to the blind 0-9 digit list.
    let tmp = empty_cwd();
    assert!(complete("bookmark go ", tmp.path()).is_none());
}
