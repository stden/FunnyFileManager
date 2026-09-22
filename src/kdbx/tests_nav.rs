//! Cursor-movement tests: clamped at both ends, never wrapping.

use super::tests_support::{password_for, selected_title, unlocked_sample, visible_titles};
use super::Motion;

#[test]
fn next_and_prev_clamp_at_both_ends_without_wrapping() {
    let (_dir, mut session, now) = unlocked_sample();
    assert_eq!(session.vault().expect("unlocked").selected(), 0);

    // At the top, Prev stays put — it does not wrap round to the bottom.
    session.move_selection(Motion::Prev, now);
    assert_eq!(session.vault().expect("unlocked").selected(), 0);
    assert_eq!(
        selected_title(&session),
        visible_titles(&session).first().cloned()
    );

    for expected in 1..=4 {
        session.move_selection(Motion::Next, now);
        assert_eq!(session.vault().expect("unlocked").selected(), expected);
    }

    // At the bottom, Next stays put — it does not wrap round to the top.
    session.move_selection(Motion::Next, now);
    assert_eq!(session.vault().expect("unlocked").selected(), 4);
    assert_eq!(
        selected_title(&session),
        visible_titles(&session).last().cloned()
    );
}

#[test]
fn paging_parks_on_the_last_row_and_on_the_first() {
    let (_dir, mut session, now) = unlocked_sample();

    session.move_selection(Motion::PageDown(3), now);
    assert_eq!(session.vault().expect("unlocked").selected(), 3);
    session.move_selection(Motion::PageDown(3), now);
    assert_eq!(session.vault().expect("unlocked").selected(), 4);
    // Parked, not wrapped.
    session.move_selection(Motion::PageDown(3), now);
    assert_eq!(session.vault().expect("unlocked").selected(), 4);

    session.move_selection(Motion::PageUp(3), now);
    assert_eq!(session.vault().expect("unlocked").selected(), 1);
    session.move_selection(Motion::PageUp(3), now);
    assert_eq!(session.vault().expect("unlocked").selected(), 0);
    session.move_selection(Motion::PageUp(3), now);
    assert_eq!(session.vault().expect("unlocked").selected(), 0);
}

#[test]
fn moving_over_an_empty_match_list_is_a_no_op() {
    let (_dir, mut session, now) = unlocked_sample();
    session
        .vault_mut()
        .expect("unlocked")
        .set_filter("zzz", now);

    session.move_selection(Motion::Next, now);
    session.move_selection(Motion::PageDown(10), now);
    session.move_selection(Motion::Prev, now);

    assert_eq!(session.vault().expect("unlocked").selected(), 0);
    assert!(selected_title(&session).is_none());
}

#[test]
fn moving_the_cursor_hides_a_revealed_password() {
    let (_dir, mut session, now) = unlocked_sample();
    let title = selected_title(&session).expect("a selection");
    assert!(session.reveal(now).expect("reveal"));
    assert_eq!(
        session.vault().expect("unlocked").revealed(),
        Some(password_for(&title))
    );

    session.move_selection(Motion::Next, now);

    // The revealed secret belonged to the previous row; leaving it on screen
    // against a different entry would be both wrong and a leak.
    assert!(session.vault().expect("unlocked").revealed().is_none());
    assert!(!session.vault().expect("unlocked").is_revealed());
}

#[test]
fn editing_the_filter_hides_a_revealed_password() {
    let (_dir, mut session, now) = unlocked_sample();
    // "git" already narrows to GitHub, and "gith" still does — so the reveal is
    // dropped because the filter changed, not because the cursor moved.
    session
        .vault_mut()
        .expect("unlocked")
        .set_filter("git", now);
    assert_eq!(selected_title(&session).as_deref(), Some("GitHub"));
    assert!(session.reveal(now).expect("reveal"));
    assert_eq!(session.vault().expect("unlocked").revealed(), Some("gh-pw"));

    session.type_char('h', now);
    assert_eq!(selected_title(&session).as_deref(), Some("GitHub"));

    assert!(session.vault().expect("unlocked").revealed().is_none());
}
