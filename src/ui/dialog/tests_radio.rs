//! Purpose: A radio row keeps its options apart, and is sized as it is drawn.

use ratatui::style::Style;

use super::super::Cells;
use super::controls::radio_line;
use super::row::Row;

const OPTIONS: [&str; 3] = ["Default", "Copy", "&Inherit"];

fn drawn(selected: usize) -> String {
    radio_line(
        "Access rights:",
        &OPTIONS,
        selected,
        false,
        Style::default(),
    )
    .spans
    .iter()
    .map(|span| span.content.as_ref())
    .collect()
}

/// The F5 dialog read `(•) Default( ) Copy( ) Inherit`, each option run
/// into the marker of the next.
#[test]
fn a_space_stands_between_radio_options() {
    assert_eq!(drawn(0), "Access rights: (•) Default ( ) Copy ( ) Inherit");
    assert_eq!(drawn(2), "Access rights: ( ) Default ( ) Copy (•) Inherit");
}

#[test]
fn a_radio_row_is_as_wide_as_it_is_drawn() {
    let row = Row::Radio {
        caption: "Access rights:",
        options: &OPTIONS,
        selected: 0,
        focused: false,
    };
    assert_eq!(row.width(), Cells::of(&drawn(0)));
}
