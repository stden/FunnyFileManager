//! Mouse clicks focus and activate the same filter controls as the keyboard.

use super::*;
use crate::models::FilterField;

#[test]
fn filter_rows_and_buttons_set_their_own_focus() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    crate::ops::filter::open(&mut app);

    click_dialog_row(&mut app, 1);
    assert_eq!(app.filter_input.field, FilterField::Exclude);
    click_dialog_row(&mut app, 2);
    assert_eq!(app.filter_input.field, FilterField::Hidden);
    assert!(matches!(&app.overlay, Overlay::Filter(prompt) if prompt.hide_dotfiles));
    click_dialog_row(&mut app, 0);
    assert_eq!(app.filter_input.field, FilterField::Include);

    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    assert_eq!(app.filter_input.field, FilterField::Cancel);
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    assert_eq!(app.filter_input.field, FilterField::Apply);
}

#[test]
fn rendered_controls_apply_the_values_selected_by_mouse() {
    let dir = TempDir::new().expect("temp");
    std::fs::write(dir.path().join("remove.rs"), b"test").expect("fixture");
    let mut app = app_at(&dir);
    let mut term = terminal();
    crate::ops::filter::open(&mut app);
    click_text(&mut term, &mut app, "Exclude");
    assert_eq!(app.filter_input.field, FilterField::Exclude);
    for ch in "*.rs".chars() {
        popup::process_popup_input(&mut app, KeyCode::Char(ch).into());
    }
    click_text(&mut term, &mut app, "Hide dotfiles");
    click_text(&mut term, &mut app, "Apply");
    assert!(!app.overlay.is_active());
    assert!(app.left_panel.filter.hide_dotfiles);
    assert!(!app
        .left_panel
        .entries
        .iter()
        .any(|entry| entry.name == "remove.rs"));
}
