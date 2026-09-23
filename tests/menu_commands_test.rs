//! F9 selection reaches real modal operations through the production dispatcher.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::{
    menu::{Heading, Menu},
    models::{App, Overlay, Panel, SortMode, SortRow},
    popup::process_popup_input,
};
use std::{fs, io};
use tempfile::TempDir;

fn enter(app: &mut App) {
    process_popup_input(app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
}

#[test]
fn every_menu_item_is_selectable() {
    for heading in Heading::ALL {
        for entry in Menu::open(heading).entries() {
            if !matches!(entry, funny_file_manager::menu::Entry::Separator) {
                assert!(
                    entry.is_selectable(),
                    "{}",
                    funny_file_manager::menu::label_of(entry)
                );
            }
        }
    }
}

#[test]
fn choosing_the_right_panel_menu_changes_only_the_right_panel() -> io::Result<()> {
    let temp = TempDir::new()?;
    let panel = Panel::new(temp.path().to_owned())?;
    let mut app = App::from_panels(panel.clone(), panel);
    select_menu(&mut app, Heading::Right, "Show long names");
    assert!(app.right_panel.options.short_names);
    assert!(!app.left_panel.options.short_names);
    Ok(())
}

#[test]
fn every_printed_extended_shortcut_resolves_to_its_menu_command() {
    for heading in Heading::ALL {
        for entry in Menu::open(heading).entries() {
            let funny_file_manager::menu::Entry::Item { shortcut, .. } = entry else {
                continue;
            };
            let Some(command) = entry.menu_command() else {
                continue;
            };
            if shortcut.is_empty() {
                continue;
            }
            let Some(key) = printed_shortcut(shortcut) else {
                panic!("{shortcut} does not name a key");
            };
            assert_eq!(
                funny_file_manager::action::resolve(key),
                funny_file_manager::action::Action::MenuCommand(command),
                "{shortcut}"
            );
        }
    }
}

/// The key a menu prints as `text` (`Ctrl+F3`, `Del`), or `None` if it names none.
fn printed_shortcut(text: &str) -> Option<KeyEvent> {
    let (modifier, key) = text.split_once('+').unwrap_or(("", text));
    let modifiers = match modifier {
        "Ctrl" => KeyModifiers::CONTROL,
        "Alt" => KeyModifiers::ALT,
        "Shift" => KeyModifiers::SHIFT,
        "" => KeyModifiers::NONE,
        _ => return None,
    };
    let code = if let Some(number) = key.strip_prefix('F') {
        KeyCode::F(number.parse().ok()?)
    } else if key == "Del" {
        KeyCode::Delete
    } else {
        KeyCode::Char(key.chars().next()?.to_ascii_lowercase())
    };
    Some(KeyEvent::new(code, modifiers))
}

#[expect(
    clippy::expect_used,
    reason = "test fixture: a menu without the named entry fails the test that chose it"
)]
fn select_menu(app: &mut App, heading: Heading, label: &str) {
    let mut menu = Menu::open(heading);
    menu.row = menu
        .entries()
        .iter()
        .position(|entry| funny_file_manager::menu::label_of(entry).replace('&', "") == label)
        .expect("menu entry exists");
    app.set_menu(menu);
    enter(app);
}

#[test]
fn sort_menu_opens_choices_and_changes_actual_file_order() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("small"), "a")?;
    fs::write(temp.path().join("large"), "larger")?;
    let mut app = App::new()?;
    app.left_panel = Panel::new(temp.path().to_owned())?;
    select_menu(&mut app, Heading::Left, "Sort modes");
    let Overlay::SortModes(menu) = &mut app.overlay else {
        panic!("sort menu must open");
    };
    // Size is the fourth order Far lists, after name, extension and time.
    menu.cursor = SortRow::ALL
        .iter()
        .position(|row| *row == SortRow::Mode(SortMode::Size))
        .expect("Size is one of the orders");
    enter(&mut app);
    assert_eq!(app.left_panel.sort_mode, SortMode::Size);
    assert_eq!(app.left_panel.entries[1].name, "large");
    Ok(())
}
