//! Reference terminal colours shared by SVG review artifacts and UI quality tests.

use ratatui::{
    buffer::Cell,
    style::{Color, Modifier},
};

const ANSI: [[u8; 3]; 16] = [
    [0, 0, 0],
    [128, 0, 0],
    [0, 128, 0],
    [128, 128, 0],
    [0, 0, 128],
    [128, 0, 128],
    [0, 128, 128],
    [192, 192, 192],
    [128, 128, 128],
    [255, 0, 0],
    [0, 255, 0],
    [255, 255, 0],
    [0, 0, 255],
    [255, 0, 255],
    [0, 255, 255],
    [255, 255, 255],
];

/// Resolve terminal defaults before reversing the effective foreground/background.
pub fn effective(cell: &Cell) -> ([u8; 3], [u8; 3]) {
    let foreground = rgb(cell.fg, [255, 255, 255]);
    let background = rgb(cell.bg, [0, 0, 0]);
    if cell.modifier.contains(Modifier::REVERSED) {
        (background, foreground)
    } else {
        (foreground, background)
    }
}

/// ANSI names and indexed colours use the reference palette, not a host theme.
pub fn rgb(color: Color, reset: [u8; 3]) -> [u8; 3] {
    match color {
        Color::Reset => reset,
        Color::Rgb(red, green, blue) => [red, green, blue],
        Color::Indexed(index) => indexed(index),
        named => named_rgb(named),
    }
}

fn named_rgb(color: Color) -> [u8; 3] {
    let index = match color {
        Color::Red => 1,
        Color::Green => 2,
        Color::Yellow => 3,
        Color::Blue => 4,
        Color::Magenta => 5,
        Color::Cyan => 6,
        Color::Gray => 7,
        Color::DarkGray => 8,
        Color::LightRed => 9,
        Color::LightGreen => 10,
        Color::LightYellow => 11,
        Color::LightBlue => 12,
        Color::LightMagenta => 13,
        Color::LightCyan => 14,
        Color::White => 15,
        _ => 0,
    };
    ANSI.get(index).copied().unwrap_or_default()
}

fn indexed(index: u8) -> [u8; 3] {
    match index {
        0..=15 => ANSI.get(usize::from(index)).copied().unwrap_or_default(),
        16..=231 => {
            let cube = index - 16;
            [level(cube / 36), level((cube / 6) % 6), level(cube % 6)]
        }
        _ => [8 + (index - 232) * 10; 3],
    }
}

fn level(value: u8) -> u8 {
    if value == 0 {
        0
    } else {
        55 + value * 40
    }
}
