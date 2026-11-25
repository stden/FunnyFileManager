//! Purpose: Verify the DNS record form leaves the cursor unset when a non-text control is focused.

use ratatui::{backend::TestBackend, layout::Position, Terminal};

use super::render;
use crate::models::cloudflare_dns::{CloudflareDnsPanel, FormField, RecordForm, Zone, ZoneId};
use crate::mouse::DialogMouse;

#[test]
fn proxy_checkbox_focus_does_not_place_the_text_caret() {
    let zone = Zone {
        id: ZoneId::new("0000000000000000000000000000000a"),
        name: "example.com".to_owned(),
    };
    let mut form = RecordForm::new_record(zone, "host".to_owned(), String::new(), String::new());
    form.field = FormField::Proxy;
    let panel = CloudflareDnsPanel::new();
    let mut tracker = DialogMouse::default();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
    terminal
        .draw(|frame| render(frame, &mut tracker, &form, &panel))
        .expect("draw");
    assert_eq!(
        terminal.get_cursor_position().expect("cursor"),
        Position::new(0, 0)
    );
}
