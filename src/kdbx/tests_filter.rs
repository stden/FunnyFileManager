//! Incremental-filter tests: what narrows the list, and what happens to the
//! cursor when the list shrinks under it.

use super::tests_support::{
    index_of, selected_title, sorted_visible_titles, unlocked_sample, visible_titles, SAMPLE,
};
use super::{KdbxError, Motion};

#[test]
fn typing_narrows_the_list_to_the_matching_entry() {
    let (_dir, mut session, now) = unlocked_sample();

    for ch in "git".chars() {
        session.type_char(ch, now);
    }

    assert_eq!(session.vault().expect("unlocked").filter(), "git");
    assert_eq!(visible_titles(&session), vec!["GitHub"]);
    assert_eq!(
        session.vault().expect("unlocked").matches(),
        [index_of(&session, "GitHub")]
    );
}

#[test]
fn filtering_is_case_insensitive_on_both_the_needle_and_the_field() {
    let (_dir, mut session, now) = unlocked_sample();

    // "Router" occurs only in a mixed-case *title* — its user name and URL are
    // "admin" and "192.168.1.1" — so a lower-case needle can only match it if
    // the field is lower-cased too. A needle like "github" would be rescued by
    // the already-lower-case URL and would prove nothing.
    session
        .vault_mut()
        .expect("unlocked")
        .set_filter("router", now);
    assert_eq!(visible_titles(&session), vec!["Router"]);

    // The other direction: an upper-case needle must be folded as well.
    session
        .vault_mut()
        .expect("unlocked")
        .set_filter("ROUTER", now);
    assert_eq!(visible_titles(&session), vec!["Router"]);

    // Upper-case needle against an all-lower-case title.
    session
        .vault_mut()
        .expect("unlocked")
        .set_filter("BANK", now);
    assert_eq!(visible_titles(&session), vec!["bank"]);
}

#[test]
fn filtering_looks_at_the_user_name_and_the_url_not_only_the_title() {
    let (_dir, mut session, now) = unlocked_sample();

    // "carol" appears only as a user name...
    session
        .vault_mut()
        .expect("unlocked")
        .set_filter("carol", now);
    assert_eq!(visible_titles(&session), vec!["bank"]);

    // ...and "192.168" only inside a URL.
    session
        .vault_mut()
        .expect("unlocked")
        .set_filter("192.168", now);
    assert_eq!(visible_titles(&session), vec!["Router"]);
}

#[test]
fn a_shrinking_filter_clamps_the_cursor_onto_the_new_last_row() {
    let (_dir, mut session, now) = unlocked_sample();
    session.move_selection(Motion::PageDown(10), now);
    assert_eq!(session.vault().expect("unlocked").selected(), 4);

    // "example" matches exactly two entries — Gmail's user name and bank's URL
    // — so a cursor sitting on row 4 must land on row 1, the new last row, and
    // never dangle past the end of `matches`.
    session
        .vault_mut()
        .expect("unlocked")
        .set_filter("example", now);

    assert_eq!(session.vault().expect("unlocked").matches().len(), 2);
    assert_eq!(sorted_visible_titles(&session), vec!["Gmail", "bank"]);
    // Row 1 is the exact clamped index: the last row of the two that survived.
    assert_eq!(session.vault().expect("unlocked").selected(), 1);
    assert_eq!(
        selected_title(&session),
        visible_titles(&session).last().cloned()
    );
}

#[test]
fn a_filter_matching_nothing_leaves_nothing_selected() {
    let (_dir, mut session, now) = unlocked_sample();

    session
        .vault_mut()
        .expect("unlocked")
        .set_filter("zzz-no-such-entry", now);

    assert!(session.vault().expect("unlocked").matches().is_empty());
    assert_eq!(session.vault().expect("unlocked").selected(), 0);
    assert!(selected_title(&session).is_none());
    assert!(matches!(
        session.copy_password(now),
        Err(KdbxError::NoSelection)
    ));
    assert!(matches!(
        session.copy_username(),
        Err(KdbxError::NoSelection)
    ));
}

#[test]
fn backspacing_the_filter_restores_the_full_list() {
    let (_dir, mut session, now) = unlocked_sample();
    session.type_char('g', now);
    session.type_char('i', now);
    assert_eq!(visible_titles(&session), vec!["GitHub"]);

    session.backspace(now);
    session.backspace(now);
    assert_eq!(session.vault().expect("unlocked").filter(), "");
    assert_eq!(visible_titles(&session).len(), SAMPLE.len());

    // Backspacing past the start is harmless.
    session.backspace(now);
    assert_eq!(session.vault().expect("unlocked").filter(), "");
    assert_eq!(visible_titles(&session).len(), SAMPLE.len());
}
