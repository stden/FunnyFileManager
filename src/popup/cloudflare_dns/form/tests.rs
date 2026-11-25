use super::process;
use crate::models::cloudflare_dns::{CloudflareDnsPanel, ViewKind};
use crossterm::event::{KeyCode, KeyModifiers};

#[test]
fn non_edit_views_ignore_form_keys_other_than_esc_and_enter() {
    let mut panel = CloudflareDnsPanel::new();
    assert_eq!(panel.view.kind(), ViewKind::Browse);

    process(&mut panel, KeyCode::Char('a'), KeyModifiers::NONE);

    assert_eq!(panel.view.kind(), ViewKind::Browse);
}
