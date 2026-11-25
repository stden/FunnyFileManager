//! Purpose: Unit tests for `ServerForm::parse` error mapping on invalid extensions.

use super::{FormField, ServerForm};
use crate::config::ValueError;

#[test]
fn parse_reports_extensions_field_on_invalid_extensions() {
    let mut form = ServerForm::new_server();
    for (field, text) in [
        (FormField::Language, "zig"),
        (FormField::Command, "zls"),
        (FormField::Extensions, "zig, ZIG"),
    ] {
        form.focus_on(field);
        form.focused_line_mut()
            .expect("text field")
            .set_split(text, "");
    }
    assert_eq!(
        form.parse(|_| false),
        Err((
            FormField::Extensions,
            ValueError::ExtensionTwice("zig".to_owned())
        ))
    );
}
