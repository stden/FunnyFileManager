//! The underlined filter controls are usable through their Alt shortcuts.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{App, FilterField, Overlay};
use crate::popup::process_popup_input;

fn alt(app: &mut App, ch: char) {
    process_popup_input(app, KeyEvent::new(KeyCode::Char(ch), KeyModifiers::ALT));
}

#[test]
fn alt_keys_focus_fields_toggle_hidden_and_apply_or_cancel() {
    let temp = tempfile::TempDir::new().expect("temp");
    let panel = crate::models::Panel::new(temp.path().to_owned()).expect("panel");
    let mut app = App::from_panels(panel.clone(), panel);
    crate::ops::filter::open(&mut app);
    alt(&mut app, 'e');
    assert_eq!(app.filter_input.field, FilterField::Exclude);
    alt(&mut app, 'i');
    assert_eq!(app.filter_input.field, FilterField::Include);
    alt(&mut app, 'd');
    assert!(matches!(&app.overlay, Overlay::Filter(prompt) if prompt.hide_dotfiles));
    alt(&mut app, 'c');
    assert!(!app.overlay.is_active());
    assert!(!app.left_panel.filter.hide_dotfiles);

    crate::ops::filter::open(&mut app);
    alt(&mut app, 'd');
    alt(&mut app, 'a');
    assert!(!app.overlay.is_active());
    assert!(app.left_panel.filter.hide_dotfiles);
}

#[test]
fn disabling_escape_keeps_the_filter_open_but_f10_still_closes_it() {
    let mut app = App::new().expect("app");
    app.settings
        .set(crate::menu::SettingsPage::Dialog, "off")
        .expect("setting");
    crate::ops::filter::open(&mut app);
    process_popup_input(&mut app, KeyCode::Esc.into());
    assert!(matches!(app.overlay, Overlay::Filter(_)));
    process_popup_input(&mut app, KeyCode::F(10).into());
    assert!(!app.overlay.is_active());
}
