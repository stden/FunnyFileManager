//! Unit tests: the command line's reverse search, and the guard every helper
//! here carries.
//!
//! Each helper begins by asking whether the command line is still the overlay
//! on screen, because it is called from a dispatcher that may have closed it —
//! a `submit` that opens a dialog, a menu row that changes the mode. Every one
//! of those guards is a branch, and a helper that acted anyway would write into
//! an overlay that is no longer there.

use tempfile::TempDir;

use super::completion::{accept_ghost, ghost_acceptable};
use super::history::{history_step, restore};
use super::search::{
    cancel_search, edit_query, finish_search, next_search_start, search_from, start_search,
};
use super::*;
use crate::history::RingIndex;
use crate::models::Panel;
use crossterm::event::{KeyCode, KeyModifiers};

mod reverse_search;

fn app_at(dir: &TempDir) -> App {
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("left");
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("right");
    app
}

fn ctrl(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL)
}

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::empty())
}

fn typed(app: &App) -> String {
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("the command line is open");
    };
    state.line.text().to_owned()
}

/// A command line with `history` behind it, ready to search.
fn app_with_history(dir: &TempDir, history: &[&str]) -> App {
    let mut app = app_at(dir);
    for line in history {
        app.history.record(line);
    }
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    app
}

// ─── the guards ─────────────────────────────────────────────────────────────

/// Every helper must be inert once the command line is no longer the overlay.
/// They are called from a dispatcher that can have closed it a moment earlier,
/// and one that acted anyway would write into a dialog that is gone.
#[test]
fn every_helper_does_nothing_once_the_command_line_has_closed() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::Help);

    assert!(state(&app).is_none());
    assert!(state_mut(&mut app).is_none());
    assert!(!menu_open(&app));
    assert!(!ghost_acceptable(&app));
    assert_eq!(next_search_start(&app), RingIndex::NEWEST);

    // None of these may panic, and none may change the overlay. The closures
    // panic on purpose: reaching one would mean the guard let it through.
    close_menu(&mut app);
    with_menu(&mut app, |_| panic!("there is no menu"));
    edit(&mut app, |_| panic!("there is no buffer"));
    move_caret(&mut app, |_| panic!("there is no caret"));
    edit_and_refresh(&mut app, |_| panic!("there is no buffer"));
    edit_query(&mut app, |_| panic!("there is no query"));
    refresh_after_edit(&mut app);
    history_step(&mut app, Step::Older);
    restore(&mut app, "anything");
    start_search(&mut app);
    search_from(&mut app, RingIndex::NEWEST);
    finish_search(&mut app);
    cancel_search(&mut app);
    complete_at_cursor(&mut app);
    refresh_menu(&mut app);
    accept_ghost(&mut app);
    refresh_ghost(&mut app);

    assert!(matches!(app.overlay, Overlay::Help), "still the help page");
}

#[test]
fn completion_is_idle_when_the_setting_is_off() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_with_history(&dir, &["git status"]);
    app.settings
        .set(crate::menu::SettingsPage::AutoComplete, "off")
        .expect("setting");
    process(&mut app, plain(KeyCode::Char('g')));
    process(&mut app, plain(KeyCode::Tab));
    assert_eq!(typed(&app), "g");
}

#[test]
fn ctrl_o_toggles_console_view_while_editing_command_line() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_with_history(&dir, &[]);
    process(&mut app, plain(KeyCode::Char('g')));
    assert_eq!(app.view, crate::models::ViewMode::Panels);
    process(&mut app, ctrl('o'));
    assert_eq!(app.view, crate::models::ViewMode::Console);
    assert_eq!(typed(&app), "g");
    process(&mut app, ctrl('щ'));
    assert_eq!(app.view, crate::models::ViewMode::Panels);
    assert_eq!(typed(&app), "g");
}
