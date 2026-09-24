//! Purpose: Pin the Language servers dialog on screen: the rows and columns
//! at 80×24, the colours §6 of the design gives them, and small screens.

use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{buffer::Buffer, style::Modifier, Terminal};

use super::super::palette;
use crate::config::document::FILE_NAME;
use crate::models::{settings::Settings, App};
use crate::test_support::lsp_machine::{editor_mut, press, press_with, select, LspMachine};
use crate::test_support::{buffer_string, framed_row, last_component};
use crate::WritableTestBackend;

pub(super) fn draw(app: &mut App, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(WritableTestBackend::new(width, height)).expect("terminal");
    terminal.draw(|f| crate::ui::ui(f, app)).expect("draw");
    terminal.backend().buffer().clone()
}

/// The cell where `needle` first starts on screen.
pub(super) fn find(buffer: &Buffer, needle: &str) -> (u16, u16) {
    let area = buffer.area;
    for y in 0..area.height {
        let row: Vec<&str> = (0..area.width).map(|x| buffer[(x, y)].symbol()).collect();
        let joined: String = row.concat();
        if let Some(byte) = joined.find(needle) {
            let column = joined.get(..byte).expect("a char boundary").chars().count();
            return (u16::try_from(column).expect("column"), y);
        }
    }
    panic!("{needle:?} is not on screen:\n{}", buffer_string(buffer));
}

/// The cell where `needle` first starts on a row below `y`.
pub(super) fn find_below(buffer: &Buffer, needle: &str, y: u16) -> Option<(u16, u16)> {
    let area = buffer.area;
    (y + 1..area.height).find_map(|row| {
        let joined: String = (0..area.width).map(|x| buffer[(x, row)].symbol()).collect();
        let byte = joined.find(needle)?;
        Some((
            u16::try_from(joined.get(..byte)?.chars().count()).ok()?,
            row,
        ))
    })
}

pub(super) fn fg(buffer: &Buffer, needle: &str) -> ratatui::style::Color {
    buffer[find(buffer, needle)].fg
}

/// A changed python command (not installed, so its recipe runs instead), a
/// disabled rust, and an added `aaa` that also claims `.ts`.
fn machine_with_changes() -> (LspMachine, App) {
    let machine = LspMachine::new();
    machine.write_settings(
        "lsp:\n  python: {command: custom}\n  rust: {enabled: false}\n  \
         aaa: {command: aaa-server, extensions: [ts], install: null}\n",
    );
    let app = machine.opened();
    (machine, app)
}

/// Whether `row` is its `Settings: ` caption and then `path`: whole, or cut
/// from its head to a tail that still ends in the file's name.
fn quotes_path(row: &str, path: &Path) -> bool {
    let path = path.display().to_string();
    let file = last_component(FILE_NAME);
    row.strip_prefix("Settings: ").is_some_and(|shown| {
        shown == path
            || shown
                .strip_prefix('…')
                .is_some_and(|tail| path.ends_with(tail) && tail.ends_with(&file))
    })
}

#[test]
fn the_list_screen_has_its_rows_labels_legend_and_key_bar() {
    let (machine, mut app) = machine_with_changes();
    select(&mut app, "python");
    let screen = buffer_string(&draw(&mut app, 80, 24));
    assert!(screen.contains(" Language servers "), "{screen}");
    assert!(
        quotes_path(&framed_row(&screen, "Settings: "), &machine.settings()),
        "{screen}"
    );
    for label in ["Command:", "Arguments:", "Status:", "Auto-install:"] {
        assert!(screen.contains(label), "{label}\n{screen}");
    }
    assert!(screen.contains("▸ python"), "{screen}");
    assert!(screen.contains("recipe"), "{screen}");
    assert!(screen.contains("Enter Edit  Space On/Off  Ins Add  Del Reset  Esc Close"));
    assert!(screen.contains("2Save"), "{screen}");
    assert!(screen.contains("8Reset"), "{screen}");
    assert!(screen.contains("No changes"), "{screen}");
}

#[test]
fn an_added_server_offers_remove_and_a_change_is_marked() {
    let (_machine, mut app) = machine_with_changes();
    select(&mut app, "aaa");
    press(&mut app, KeyCode::Char(' '));
    let buffer = draw(&mut app, 80, 24);
    let screen = buffer_string(&buffer);
    assert!(screen.contains("Del Remove"), "{screen}");
    assert!(screen.contains("8Remove"), "{screen}");
    assert!(
        screen.contains("Modified: aaa. F2 saves only what changed."),
        "{screen}"
    );
    let (x, y) = find(&buffer, "▸ aaa");
    assert_eq!(buffer[(x + 19, y)].symbol(), "*", "{screen}");
}

#[test]
fn small_screens_draw_without_panicking_and_drop_the_detail() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    let screen = buffer_string(&draw(&mut app, 60, 16));
    assert!(screen.contains("Command:"), "{screen}");
    let screen = buffer_string(&draw(&mut app, 40, 10));
    assert!(!screen.contains("Command:"), "{screen}");
    assert!(screen.contains("F3 shows details"), "{screen}");
    for (width, height) in [(10, 3), (4, 3), (1, 1), (0, 0)] {
        drop(draw(&mut app, width, height));
    }
    press(&mut app, KeyCode::F(3));
    for (width, height) in [(40, 10), (10, 4), (1, 1)] {
        drop(draw(&mut app, width, height));
    }
}

#[path = "tests_form.rs"]
mod form;

#[path = "tests_capture.rs"]
mod capture;

#[path = "tests_layout.rs"]
mod layout;

#[path = "tests_wrap.rs"]
mod wrap;

#[path = "tests_colours.rs"]
mod colours;

#[path = "tests_status_rows.rs"]
mod status_rows;

#[path = "tests_keys.rs"]
mod keys;

#[path = "tests_form_controls.rs"]
mod form_controls;
