//! The panels explain available actions without changing their key layout.

use ratatui::{backend::TestBackend, Terminal};

use super::render_widths::{blank_app, left_panel_text, row_text, screen_text};
use super::*;
use crate::models::{DirEntry, EntryKind};

fn drawn(app: &mut App, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
    terminal.draw(|frame| ui(frame, app)).expect("render");
    terminal
}

#[test]
fn help_mentions_quick_view_and_filter_on_a_standard_terminal() {
    let (mut app, _temp) = blank_app();
    app.set_overlay(Overlay::Help);
    let screen = screen_text(&drawn(&mut app, 80, 24));
    assert!(screen.contains("Ctrl+Q"), "{screen}");
    assert!(
        screen.contains("F9 > Commands > File panel filter"),
        "{screen}"
    );
    assert!(
        screen.contains(":winmsg"),
        "the end of help is visible: {screen}"
    );
}

#[test]
fn active_filter_is_explained_alongside_the_selected_entry() {
    let (mut app, _temp) = blank_app();
    app.left_panel.entries.push(DirEntry {
        name: "visible.txt".to_owned(),
        kind: EntryKind::File,
        ..DirEntry::default()
    });
    app.left_panel.select(Some(0));
    app.left_panel.filter.hide_dotfiles = true;
    app.left_panel.filter.include = Some(crate::mask::Mask::parse("*.RS"));
    let screen = left_panel_text(&drawn(&mut app, 80, 24));
    assert!(screen.contains("Filter: *.RS"), "{screen}");
    assert!(screen.contains("visible.txt"), "{screen}");
}

#[test]
fn an_empty_folder_offers_creation_without_covering_the_parent_row() {
    let (mut app, _temp) = blank_app();
    app.left_panel.entries.push(DirEntry {
        name: "..".to_owned(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    });
    app.left_panel.select(Some(0));
    let screen = left_panel_text(&drawn(&mut app, 80, 24));
    assert!(screen.contains("No visible entries"), "{screen}");
    assert!(screen.contains("F7"), "{screen}");
    assert!(screen.contains(".."), "{screen}");
}

#[test]
fn an_empty_filtered_listing_explains_why_no_entries_are_shown() {
    let (mut app, _temp) = blank_app();
    app.left_panel.filter.hide_dotfiles = true;
    let screen = left_panel_text(&drawn(&mut app, 80, 24));
    assert!(screen.contains("No matches"), "{screen}");
    assert!(screen.contains("F9 > Commands"), "{screen}");
    assert!(!screen.contains("F7"), "{screen}");
}

#[test]
fn legacy_panel_mask_is_visible_and_identifies_an_empty_result() {
    let (mut app, _temp) = blank_app();
    app.left_panel.options.filter = "*.RS".to_owned();
    let screen = left_panel_text(&drawn(&mut app, 80, 24));
    assert!(screen.contains("Filter: *.RS"), "{screen}");
    assert!(screen.contains("No matches"), "{screen}");
    assert!(screen.contains("F9 > Commands"), "{screen}");
}

#[test]
fn the_active_mask_remains_visible_beside_a_long_selected_filename() {
    let (mut app, _temp) = blank_app();
    app.left_panel.entries.push(DirEntry {
        name: "a-very-long-filename-selected-for-a-filtered-list.txt".to_owned(),
        kind: EntryKind::File,
        ..DirEntry::default()
    });
    app.left_panel.select(Some(0));
    app.left_panel.options.filter = "*.RS".to_owned();
    let screen = left_panel_text(&drawn(&mut app, 80, 24));
    assert!(screen.contains("Filter: *.RS"), "{screen}");
}

#[test]
fn help_scrolls_on_a_short_terminal_and_keeps_the_close_keys() {
    let (mut app, _temp) = blank_app();
    app.set_viewport(crate::models::Viewport::new(12));
    app.set_overlay(Overlay::Help);
    let first = screen_text(&drawn(&mut app, 80, 12));
    assert!(first.contains("PgUp"), "{first}");
    assert!(first.contains("Ctrl+Q"), "{first}");
    assert!(!first.contains(":winmsg"), "{first}");
    for _ in 0..2 {
        crate::popup::process_popup_input(&mut app, crossterm::event::KeyCode::PageDown.into());
    }
    let last = screen_text(&drawn(&mut app, 80, 12));
    assert!(last.contains(":winmsg"), "{last}");
    crate::popup::process_popup_input(&mut app, crossterm::event::KeyCode::Home.into());
    let first_again = screen_text(&drawn(&mut app, 80, 12));
    assert!(first_again.contains("Ctrl+Q"), "{first_again}");
    crate::popup::process_popup_input(&mut app, crossterm::event::KeyCode::F(1).into());
    assert!(!app.overlay.is_active());
}

#[test]
fn panel_key_bar_uses_readable_labels_at_both_widths() {
    let (mut app, _temp) = blank_app();
    let narrow = row_text(&drawn(&mut app, 80, 24), 23);
    assert!(narrow.contains("2User"), "{narrow}");
    assert!(narrow.contains("6Move"), "{narrow}");
    assert!(narrow.contains("7MkDir"), "{narrow}");

    let wide = row_text(&drawn(&mut app, 160, 24), 23);
    assert!(wide.contains("2User menu"), "{wide}");
    assert!(wide.contains("6Move/Rename"), "{wide}");
    assert!(wide.contains("7New folder"), "{wide}");
}

#[test]
fn filter_dialog_shows_both_independent_include_masks() {
    let (mut app, _temp) = blank_app();
    app.left_panel.options.filter = "*.rs".to_owned();
    app.left_panel.filter.include = Some(crate::mask::Mask::parse("*report*"));
    crate::ops::filter::open(&mut app);
    let screen = screen_text(&drawn(&mut app, 80, 24));
    assert!(screen.contains("Include:"), "{screen}");
    assert!(screen.contains("Extra include:"), "{screen}");
    assert!(screen.contains("*.rs"), "{screen}");
    assert!(screen.contains("*report*"), "{screen}");
}

#[test]
fn a_failed_filter_refresh_displays_the_error_in_the_open_dialog() {
    let (mut app, temp) = blank_app();
    crate::ops::filter::open(&mut app);
    std::fs::remove_dir_all(temp.path()).expect("remove fixture");
    crate::popup::process_popup_input(&mut app, crossterm::event::KeyCode::Enter.into());
    let screen = screen_text(&drawn(&mut app, 80, 24));
    assert!(screen.contains("Cannot refresh folder:"), "{screen}");
    assert!(matches!(app.overlay, Overlay::Filter(_)));
}

#[test]
fn a_narrow_panel_keeps_the_selected_name_when_the_filter_hint_cannot_fit() {
    let (mut app, _temp) = blank_app();
    app.left_panel.entries.push(DirEntry {
        name: "doc".to_owned(),
        kind: EntryKind::File,
        ..DirEntry::default()
    });
    app.left_panel.select(Some(0));
    app.left_panel.options.filter = "*.rs".to_owned();
    let screen = left_panel_text(&drawn(&mut app, 30, 12));
    assert!(screen.contains("doc"), "{screen}");
}

#[test]
fn help_end_up_and_page_up_navigate_without_closing_it() {
    let (mut app, _temp) = blank_app();
    app.set_viewport(crate::models::Viewport::new(12));
    app.set_overlay(Overlay::Help);
    crate::popup::process_popup_input(&mut app, crossterm::event::KeyCode::End.into());
    let end = screen_text(&drawn(&mut app, 80, 12));
    assert!(end.contains(":winmsg"), "{end}");
    crate::popup::process_popup_input(&mut app, crossterm::event::KeyCode::Up.into());
    let previous = screen_text(&drawn(&mut app, 80, 12));
    assert!(!previous.contains(":winmsg"), "{previous}");
    for _ in 0..2 {
        crate::popup::process_popup_input(&mut app, crossterm::event::KeyCode::PageUp.into());
    }
    let first = screen_text(&drawn(&mut app, 80, 12));
    assert!(first.contains("Ctrl+Q"), "{first}");
    assert!(matches!(app.overlay, Overlay::Help));
}
