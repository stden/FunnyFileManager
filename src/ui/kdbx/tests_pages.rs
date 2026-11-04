//! Purpose: A read-only vault says so in its title, and a delete confirmation
//! takes the place of the entry browser.

use super::tests::draw;
use crate::kdbx::tests_support::unlocked_sample;
use crate::kdbx::vault::{BrowsePage, Page};
use crate::kdbx::Access;

#[test]
fn a_read_only_vault_says_view_in_its_title() {
    let (_dir, mut session, _now) = unlocked_sample();
    session.access = Access::View;
    assert!(draw(&session).contains("[View]"));
}

#[test]
fn a_delete_confirmation_takes_the_place_of_the_browser() {
    let (_dir, mut session, _now) = unlocked_sample();
    session.vault_mut().expect("an unlocked vault").page = Page::DeleteConfirm(BrowsePage::Entries);
    let screen = draw(&session);
    assert!(screen.contains("Delete"), "{screen}");
}

#[test]
fn entry_form_renders_validation_error_banner() {
    let (_dir, mut session, now) = unlocked_sample();
    session
        .vault_mut()
        .expect("unlocked")
        .set_filter("GitHub", now);
    session.begin_edit(now).expect("edit form");
    for _ in "GitHub".chars() {
        session.backspace(now);
    }
    for ch in "Gmail".chars() {
        session.type_char(ch, now);
    }
    assert!(!session.save_edit(now));
    let screen = draw(&session);
    assert!(
        screen.contains("An entry with that title already exists"),
        "{screen}"
    );
}
