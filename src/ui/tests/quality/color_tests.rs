//! All declared ANSI names and representative extended colour codes resolve consistently.

use ratatui::style::Color;

use super::colors::rgb;

#[test]
fn named_and_indexed_ansi_colours_share_the_declared_reference_palette() {
    let named = [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::Gray,
        Color::DarkGray,
        Color::LightRed,
        Color::LightGreen,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
        Color::White,
    ];
    for (index, name) in named.into_iter().enumerate() {
        assert_eq!(
            rgb(name, [0; 3]),
            rgb(Color::Indexed(u8::try_from(index).unwrap()), [0; 3])
        );
    }
}

#[test]
fn extended_colour_cube_and_ramp_have_the_expected_intensities() {
    for (index, expected) in [
        (16, [0, 0, 0]),
        (17, [0, 0, 95]),
        (21, [0, 0, 255]),
        (22, [0, 95, 0]),
        (51, [0, 255, 255]),
        (52, [95, 0, 0]),
        (196, [255, 0, 0]),
        (231, [255, 255, 255]),
        (232, [8, 8, 8]),
        (255, [238, 238, 238]),
    ] {
        assert_eq!(rgb(Color::Indexed(index), [0; 3]), expected);
    }
}
