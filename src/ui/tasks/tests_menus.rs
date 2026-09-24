//! Unit tests for the task list's menus: every row drawn, the tick on the
//! choice in force, and every shortcut shown one the keymap answers to.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::menus::MenuView;
use super::tests::sketch;
use crate::action::tasks::{resolve, TaskCommand};
use crate::models::{Modal, SortKey, TaskMenu};

/// The menu open on the sketch after `key`, nothing marked.
fn menu_after(key: KeyCode) -> MenuView {
    let mut app = sketch();
    app.list().unmark_all();
    app.press(key);
    let list = app.list();
    let Modal::Menu(menu) = list.modal() else {
        panic!("no menu open");
    };
    MenuView::of(list, menu)
}

/// The key a menu shortcut names: `P`, or `Ctrl+F11`.
fn key_named(name: &str) -> KeyEvent {
    match name.strip_prefix("Ctrl+F") {
        Some(number) => KeyEvent::new(KeyCode::F(number.parse().unwrap()), KeyModifiers::CONTROL),
        None => KeyEvent::new(
            KeyCode::Char(name.chars().next().unwrap()),
            KeyModifiers::NONE,
        ),
    }
}

#[test]
fn the_sort_menu_ticks_the_key_in_force_and_ends_with_reverse_order() {
    let menu = menu_after(KeyCode::F(6));
    assert_eq!(menu.title, "Sort by");
    assert_eq!(menu.rows.len(), TaskMenu::SORT_ROWS);
    assert_eq!(menu.selected, 7, "on CPU%");
    assert_eq!(menu.rows[7], "√ CPU%              P");
    assert_eq!(menu.rows[0], "  PID               N");
    assert_eq!(menu.rows[2], "  Priority");
    assert_eq!(menu.rows[12], "  Reverse order     I");
}

#[test]
fn every_sort_shortcut_shown_sorts_by_its_row() {
    let menu = menu_after(KeyCode::F(6));
    let (keys, reverse) = menu.rows.split_at(SortKey::ALL.len());
    for (row, key) in keys.iter().zip(SortKey::ALL) {
        let Some((_, shortcut)) = row.rsplit_once("   ") else {
            continue;
        };
        let command = resolve(key_named(shortcut.trim()));
        assert_eq!(command, TaskCommand::SortBy(key), "{row}");
    }
    let (_, invert) = reverse[0].rsplit_once("   ").unwrap();
    assert_eq!(resolve(key_named(invert.trim())), TaskCommand::InvertOrder);
}

#[test]
fn the_user_menu_counts_each_owners_processes_and_ticks_the_scope() {
    let menu = menu_after(KeyCode::Char('u'));
    assert_eq!(menu.title, "Show processes of");
    assert_eq!(menu.rows, ["√ All users", "  bob (3)", "  root (2)"]);
    let mut app = sketch();
    app.press(KeyCode::Char('u'));
    app.press(KeyCode::Down);
    app.press(KeyCode::Enter);
    app.press(KeyCode::Char('u'));
    let list = app.list();
    let Modal::Menu(menu) = list.modal() else {
        panic!("no menu open");
    };
    let view = MenuView::of(list, menu);
    assert_eq!(view.rows, ["  All users", "√ bob (3)", "  root (2)"]);
    assert_eq!(view.selected, 1);
}

#[cfg(unix)]
#[test]
fn the_signal_menu_is_titled_after_its_targets() {
    let menu = menu_after(KeyCode::F(9));
    assert_eq!(menu.title, "Send signal to 4242 firefox");
    assert_eq!(menu.rows[0], "SIGTERM    ask to terminate");
    assert!(menu.rows.iter().all(|row| row.starts_with("SIG")));
    let mut app = sketch();
    app.press(KeyCode::Char('k'));
    let list = app.list();
    let Modal::Menu(menu) = list.modal() else {
        panic!("no menu open");
    };
    assert_eq!(MenuView::of(list, menu).title, "Send signal to 2 processes");
}
