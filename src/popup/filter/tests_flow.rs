//! The filter dialog edits and applies the active panel's actual listing.

use std::fs;

use crossterm::event::KeyCode;
use tempfile::TempDir;

use crate::action::ToolCommand;
use crate::mask::Mask;
use crate::models::{App, Overlay, Panel};
use crate::popup::{process_popup_input, tests::key};

fn app_on(temp: &TempDir) -> App {
    for name in ["keep.rs", "drop.rs", "notes.txt", ".hidden.rs"] {
        fs::write(temp.path().join(name), b"test").expect("write fixture");
    }
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    app.left_panel.select_named("keep.rs");
    app
}

fn type_text(app: &mut App, text: &str) {
    for ch in text.chars() {
        process_popup_input(app, key(KeyCode::Char(ch)));
    }
}

fn names(app: &App) -> Vec<&str> {
    app.left_panel
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect()
}

#[test]
fn input_applies_both_masks_and_hidden_setting_and_reopens_the_same_values() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_on(&temp);
    crate::ops::filter::open(&mut app);
    type_text(&mut app, "*.RS");
    process_popup_input(&mut app, key(KeyCode::Tab));
    type_text(&mut app, "drop.rs");
    process_popup_input(&mut app, key(KeyCode::Tab));
    process_popup_input(&mut app, key(KeyCode::Char(' ')));
    process_popup_input(&mut app, key(KeyCode::Enter));

    assert!(!app.overlay.is_active());
    assert_eq!(names(&app), ["..", "keep.rs"]);
    assert_eq!(
        app.left_panel.get_selected_entry().map(|e| e.name.as_str()),
        Some("keep.rs")
    );
    crate::ops::filter::open(&mut app);
    let Overlay::Filter(dialog) = &app.overlay else {
        panic!("filter dialog")
    };
    assert_eq!(dialog.include, "*.RS");
    assert_eq!(dialog.exclude, "drop.rs");
    assert!(dialog.hide_dotfiles);
}

#[test]
fn open_preserves_quoted_masks_and_escape_does_not_change_the_filter() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_on(&temp);
    app.left_panel.filter.include = Some(Mask::parse("\"Keep,one.rs\",*.RS"));
    let before = app.left_panel.filter.clone();
    let shown = names(&app)
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    crate::ops::filter::open(&mut app);
    let Overlay::Filter(dialog) = &app.overlay else {
        panic!("filter dialog")
    };
    assert_eq!(dialog.include, "\"Keep,one.rs\",*.RS");
    type_text(&mut app, "x");
    process_popup_input(&mut app, key(KeyCode::Esc));
    assert_eq!(app.left_panel.filter, before);
    assert_eq!(names(&app), shown);
}

#[test]
fn failed_refresh_preserves_the_old_filter_and_listing() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_on(&temp);
    let before_filter = app.left_panel.filter.clone();
    let before = names(&app)
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    crate::ops::filter::open(&mut app);
    type_text(&mut app, "*.rs");
    fs::remove_dir_all(temp.path()).expect("remove fixture directory");
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert_eq!(app.left_panel.filter, before_filter);
    assert_eq!(names(&app), before);
    assert!(
        matches!(app.overlay, Overlay::Filter(_)),
        "the edit remains available"
    );
    assert_ne!(app.status_text(), "");
}

#[test]
fn commands_menu_edits_existing_mask_and_keeps_directories_available() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_on(&temp);
    fs::create_dir(temp.path().join("docs")).expect("directory");
    app.left_panel.options.filter = "*.RS".to_owned();
    app.left_panel.load_entries().expect("refresh");
    crate::ops::menu::execute_tool(&mut app, ToolCommand::Filter).expect("menu action");
    let Overlay::Filter(dialog) = &app.overlay else {
        panic!("filter dialog")
    };
    assert_eq!(dialog.include, "*.RS");
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(names(&app).contains(&"docs"), "directories stay navigable");
    crate::ops::menu::execute_tool(&mut app, ToolCommand::Filter).expect("menu action");
    let Overlay::Filter(dialog) = &app.overlay else {
        panic!("filter dialog")
    };
    assert_eq!(dialog.include, "*.RS");
}

#[test]
fn pasted_text_targets_focused_mask_and_never_edits_checkboxes_or_buttons() {
    let temp = TempDir::new().expect("temp");
    let mut app = app_on(&temp);
    crate::ops::filter::open(&mut app);
    assert!(app.focused_text_field().is_some());
    crate::popup::paste_text(&mut app, "*.файл\n");
    process_popup_input(&mut app, key(KeyCode::Tab));
    assert!(app.focused_text_field().is_some());
    crate::popup::paste_text(&mut app, "*.bak\n");
    process_popup_input(&mut app, key(KeyCode::Tab));
    assert!(app.focused_text_field().is_none());
    crate::popup::paste_text(&mut app, "ignored");
    process_popup_input(&mut app, key(KeyCode::Tab));
    assert!(app.focused_text_field().is_none());
    crate::popup::paste_text(&mut app, "ignored");
    process_popup_input(&mut app, key(KeyCode::Tab));
    assert!(app.focused_text_field().is_none());
    crate::popup::paste_text(&mut app, "ignored");
    let Overlay::Filter(dialog) = &app.overlay else {
        panic!("filter dialog")
    };
    assert_eq!(dialog.include, "*.файл");
    assert_eq!(dialog.exclude, "*.bak");
    process_popup_input(&mut app, key(KeyCode::Tab));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert_eq!(app.left_panel.options.filter, "*.файл");
    assert_eq!(
        app.left_panel.filter.exclude.as_ref().map(Mask::source),
        Some("*.bak")
    );
}
