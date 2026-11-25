//! Purpose: Unit tests for `RecordForm::backspace` when focused on non-text fields.

use super::super::{Zone, ZoneId};
use super::{FormField, RecordForm};

#[test]
fn backspace_on_proxy_field_leaves_text_fields_unchanged() {
    let zone = Zone {
        id: ZoneId::new("0000000000000000000000000000000a"),
        name: "example.com".to_owned(),
    };
    let mut form =
        RecordForm::new_record(zone, "host".to_owned(), "1.2.3.4".to_owned(), String::new());
    form.field = FormField::Proxy;
    form.backspace();
    assert_eq!(form.name, "host");
    assert_eq!(form.content, "1.2.3.4");
    assert_eq!(form.ttl, "1");
}
