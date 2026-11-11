//! Purpose: A form is checked before it is confirmed, a confirmation queues exactly one write, and a
//! write's answer returns the dialog to the form when it failed and to the list when it was saved.

use super::*;
use crate::models::cloudflare_dns::{Change, CloudflareError, FormField};

fn changed_content() -> RecordForm {
    let mut form = edit_form();
    form.content = "203.0.113.9".to_owned();
    form
}

#[test]
fn an_invalid_form_is_refused_at_review_and_stays_on_the_form() {
    let mut form = RecordForm::new_record(
        zones().remove(0),
        "a_b".to_owned(),
        "203.0.113.9".to_owned(),
        String::new(),
    );
    form.field = FormField::Name;
    let mut panel = editing(form);
    review(&mut panel);
    assert_eq!(panel.view.kind(), ViewKind::Edit);
    assert!(
        panel.status.contains("label"),
        "the refusal says what is wrong"
    );
}

#[test]
fn a_valid_form_becomes_a_confirmation_and_queues_nothing() {
    let mut panel = editing(changed_content());
    review(&mut panel);
    assert_eq!(panel.view.kind(), ViewKind::Confirm);
    assert_eq!(
        panel.queued, None,
        "nothing is written before the confirmation"
    );
}

#[test]
fn an_unchanged_record_is_not_a_confirmation() {
    let mut panel = editing(edit_form());
    review(&mut panel);
    assert_eq!(panel.view.kind(), ViewKind::Edit);
    assert_eq!(panel.status, CloudflareError::NoChange.message());
}

#[test]
fn applying_a_confirmation_queues_one_write_of_exactly_that_change() {
    let mut form = edit_form();
    form.ttl = "600".to_owned();
    let mut panel = editing(form);
    review(&mut panel);
    assert_eq!(panel.queued, None);
    apply(&mut panel);
    let Some(Request::Write(Change::Update(update))) = &panel.queued else {
        panic!("the write is queued");
    };
    assert_eq!(update.ttl, Ttl::Seconds(600));
    assert_eq!(panel.busy, Some(Busy::Saving));
}

#[test]
fn a_refused_write_returns_to_the_form_with_the_reason() {
    let mut panel = editing(changed_content());
    review(&mut panel);
    apply(&mut panel);
    publish(&mut panel, Reply::Written(Err(CloudflareError::Network)));
    assert_eq!(panel.view.kind(), ViewKind::Edit);
    assert_eq!(panel.status, CloudflareError::Network.message());
    assert_eq!(panel.busy, None);
}

#[test]
fn a_saved_write_returns_to_the_list_and_reloads_its_zone() {
    let mut panel = editing(changed_content());
    review(&mut panel);
    apply(&mut panel);
    let saved = record("www.example.com", "203.0.113.9");
    publish(&mut panel, Reply::Written(Ok(saved)));
    assert_eq!(panel.view, View::Browse);
    assert_eq!(panel.queued, Some(Request::Records(ZoneId::new(ZONE_A))));
}

#[test]
fn going_back_from_a_confirmation_keeps_the_typed_values() {
    let mut panel = editing(changed_content());
    review(&mut panel);
    back(&mut panel);
    let View::Edit(form) = &panel.view else {
        panic!("the form");
    };
    assert_eq!(form.content, "203.0.113.9");
}

#[test]
fn a_write_cannot_be_queued_over_a_request_still_in_flight() {
    let mut panel = editing(changed_content());
    review(&mut panel);
    panel.enqueue(Request::Zones);
    apply(&mut panel);
    assert_eq!(
        panel.queued,
        Some(Request::Zones),
        "the in-flight load is kept"
    );
    assert!(panel.status.contains("Still"), "the refusal is said");
}

#[test]
fn the_confirmation_names_the_record_it_changes() {
    let mut panel = editing(changed_content());
    review(&mut panel);
    let View::Confirm(_, change) = &panel.view else {
        panic!("the confirmation");
    };
    let Change::Update(update) = change.as_ref() else {
        panic!("an update");
    };
    assert_eq!(update.record.name, "www.example.com");
    assert_eq!(update.content, "203.0.113.9");
}
