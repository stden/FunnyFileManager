//! Unit tests: the key bar, Far's or a custom one, keeps every cell where the
//! mouse's hit test expects it on a narrow terminal, and draws its own labels.

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::render_widths::{blank_app, row_text};
use super::*;

/// A 20-column bar gives the last four keys a one-column cell. The key number
/// must be clipped to its cell like the label is, or every cell after "10"
/// drifts one column right of where `fkey_at` — and so a click — expects it.
#[test]
fn test_narrow_key_bar_keeps_every_cell_where_the_hit_test_expects_it() {
    let backend = TestBackend::new(NARROW_KEY_BAR, 8);
    let mut terminal = Terminal::new(backend).unwrap();
    let (mut app, _temp) = blank_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    assert_key_bar_agrees_with_the_hit_test(&terminal, &FKEY_LABELS);
}

/// Twelve labels of a full-screen view's own, as the task list passes them.
const CUSTOM_KEYS: [(&str, &str); FKEY_COUNT] = [
    ("1", "Help"),
    ("2", "Name"),
    ("3", "Info"),
    ("4", "Filter"),
    ("5", "Tree"),
    ("6", "Sort"),
    ("7", "Nice"),
    ("8", "Kill"),
    ("9", "Signal"),
    ("10", "Quit"),
    ("11", "Hold"),
    ("12", ""),
];

/// `CUSTOM_KEYS` drawn on the bottom row of a terminal `width` wide.
fn custom_key_bar(width: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, 3)).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            let bottom = Rect::new(0, area.height - 1, area.width, 1);
            render_key_bar(f, bottom, &CUSTOM_KEYS);
        })
        .unwrap();
    terminal
}

/// A full-screen view that passes its own twelve labels gets the same
/// agreement, narrow or at the classic width: the cells are laid out by the
/// hit test's arithmetic whatever the labels say.
#[test]
fn a_custom_key_bar_keeps_every_cell_where_the_hit_test_expects_it() {
    for width in [NARROW_KEY_BAR, 80] {
        assert_key_bar_agrees_with_the_hit_test(&custom_key_bar(width), &CUSTOM_KEYS);
    }
}

/// The labels a view passes are the ones on screen. The agreement test above
/// cannot tell: its key digits are Far's, so a bar that drew `FKEY_LABELS`
/// whatever it was given would pass it.
#[test]
fn a_custom_key_bar_draws_its_own_labels_not_fars() {
    let terminal = custom_key_bar(80);
    assert_eq!(
        row_text(&terminal, 2),
        "1Help  2Name  3Info  4Filter5Tree  6Sort  7Nice  8Kill  9Sign…10Quit11Hold12"
    );
}

/// The narrowest bar the agreement tests draw: the last four keys get one
/// column each.
const NARROW_KEY_BAR: u16 = 20;

/// Every key number on the bottom row starts where `fkey_at` says its cell does.
fn assert_key_bar_agrees_with_the_hit_test(
    terminal: &Terminal<TestBackend>,
    labels: &[(&str, &str); FKEY_COUNT],
) {
    let buf = terminal.backend().buffer();
    let (width, y) = (buf.area.width, buf.area.height - 1);
    let mut left = 0u16;
    let widths = crate::mouse::fkey_widths(width, labels.len());
    for (index, cell) in widths.into_iter().enumerate() {
        let (key, _) = labels[index];
        assert_eq!(
            Some(buf[(left, y)].symbol()),
            key.get(..1),
            "key {} must start at column {left}, row: {:?}",
            index + 1,
            row_text(terminal, y)
        );
        assert_eq!(
            crate::mouse::fkey_at(width, labels.len(), left),
            u8::try_from(index + 1).ok(),
            "and the hit test must agree"
        );
        left += cell;
    }
}
