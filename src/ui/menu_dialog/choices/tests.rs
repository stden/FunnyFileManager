//! Purpose: Pins that a menu of plain labels draws and hit-tests by the same geometry.

use ratatui::{backend::TestBackend, layout::Rect, Terminal};

use super::{render, row_at};
use crate::ui::palette;

const TITLE: &str = "Sort by";
const LABELS: [&str; 3] = ["CPU%", "Memory", "PID"];

/// The screen row whose text contains `label`, if any.
fn row_of(terminal: &Terminal<TestBackend>, label: &str) -> Option<u16> {
    let buf = terminal.backend().buffer();
    (0..buf.area.height).find(|&y| {
        let line: String = (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect();
        line.contains(label)
    })
}

fn drawn(screen: Rect, selected: usize) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(screen.width, screen.height)).unwrap();
    terminal
        .draw(|f| render(f, TITLE, &LABELS, selected))
        .unwrap();
    terminal
}

#[test]
fn a_menu_of_plain_labels_draws_each_label_with_the_cursor_on_the_selected_one() {
    let terminal = drawn(Rect::new(0, 0, 60, 20), 1);
    let buf = terminal.backend().buffer();
    let memory = row_of(&terminal, "Memory").expect("the selected label is drawn");
    let cpu = row_of(&terminal, "CPU%").expect("an unselected label is drawn");
    assert_eq!(row_of(&terminal, "PID"), Some(memory + 1));
    let text_x = (0..60).find(|&x| buf[(x, memory)].symbol() == "M").unwrap();
    assert_eq!(buf[(text_x, memory)].bg, palette::MENU_SELECTED_BG);
    assert_eq!(buf[(text_x, cpu)].bg, palette::MENU_BG);
}

/// A click on a drawn label answers that label's index, and a click on the
/// frame or beside the menu answers nothing.
#[test]
fn a_menu_of_plain_labels_is_hit_tested_where_it_is_drawn() {
    let screen = Rect::new(0, 0, 60, 20);
    let terminal = drawn(screen, 0);
    for (index, label) in LABELS.iter().enumerate() {
        let y = row_of(&terminal, label).unwrap();
        assert_eq!(
            row_at(screen, TITLE, &LABELS, 0, 30, y),
            Some(index),
            "{label}"
        );
    }
    let title_row = row_of(&terminal, TITLE).unwrap();
    assert_eq!(row_at(screen, TITLE, &LABELS, 0, 30, title_row), None);
    assert_eq!(row_at(screen, TITLE, &LABELS, 0, 0, 0), None);
}

/// Scrolled to the last row of a menu taller than its box, the hit test
/// answers the rows actually drawn, not the first ones.
#[test]
fn a_scrolled_menu_of_plain_labels_hit_tests_the_rows_it_shows() {
    let labels: Vec<String> = (0..40).map(|i| format!("Owner {i}")).collect();
    let screen = Rect::new(0, 0, 60, 12);
    let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
    terminal.draw(|f| render(f, "Show", &labels, 39)).unwrap();
    let last = row_of(&terminal, "Owner 39").expect("the selected row is scrolled into view");
    assert_eq!(row_at(screen, "Show", &labels, 39, 30, last), Some(39));
}
