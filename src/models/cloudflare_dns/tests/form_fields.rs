//! Purpose: The record form walks its fields in order, skips the name of an existing record, types
//! into the focused text field, and switches the proxy only on the proxy field.

use super::*;

#[test]
fn an_existing_record_form_starts_on_content_and_never_stops_on_its_name() {
    let mut form = RecordForm::edit(
        zone("0000000000000000000000000000000a", "example.com"),
        record("0000000000000000000000000000000b", "A", Proxy::Off),
    );
    assert_eq!(form.field, FormField::Content);
    assert_eq!(form.ttl, "300");
    form.step(true);
    assert_eq!(form.field, FormField::Ttl);
    form.step(true);
    assert_eq!(form.field, FormField::Proxy);
    form.step(true);
    assert_eq!(form.field, FormField::Content, "the walk wraps round");
    form.step(false);
    assert_eq!(form.field, FormField::Proxy);
}

#[test]
fn a_new_host_form_walks_name_first_and_types_into_the_focused_field() {
    let mut form = RecordForm::new_record(
        zone("0000000000000000000000000000000a", "example.com"),
        "host".to_owned(),
        String::new(),
        "Public IPv4 not detected".to_owned(),
    );
    assert_eq!(form.field, FormField::Name);
    form.type_char('2');
    assert_eq!(form.name, "host2");
    form.step(true);
    form.type_char('1');
    form.type_char('.');
    assert_eq!(form.content, "1.");
    form.backspace();
    assert_eq!(form.content, "1");
    assert_eq!(form.note, "Public IPv4 not detected");
}

#[test]
fn only_the_proxy_field_toggles_and_only_when_the_record_can_be_proxied() {
    let mut form = RecordForm::edit(
        zone("0000000000000000000000000000000a", "example.com"),
        record(
            "0000000000000000000000000000000b",
            "TXT",
            Proxy::Unsupported,
        ),
    );
    form.toggle_proxy();
    assert_eq!(
        form.proxy,
        Proxy::Unsupported,
        "the content field is not the proxy"
    );
    form.field = FormField::Proxy;
    form.toggle_proxy();
    assert_eq!(form.proxy, Proxy::Unsupported);
    form.type_char('x');
    assert_eq!(form.content, "203.0.113.5", "the proxy field takes no text");
    form.field = FormField::Content;
    form.type_char('x');
    assert_eq!(
        form.content, "203.0.113.5x",
        "text goes to the content field"
    );
}

#[test]
fn a_field_the_form_does_not_show_is_left_where_it_is() {
    let mut form = RecordForm::edit(
        zone("0000000000000000000000000000000a", "example.com"),
        record("0000000000000000000000000000000b", "A", Proxy::Off),
    );
    form.field = FormField::Name;
    form.step(true);
    assert_eq!(form.field, FormField::Name);
}
