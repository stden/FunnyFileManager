//! Nominal sRGB readability checks over rendered terminal text cells.
//!
//! The shared reference palette resolves ANSI/indexed colours and Reset.
//! Only box-drawing decoration and image/graph block glyphs are excluded.
//! Disabled controls, DIM and hidden-file labels remain readable product text.

use std::collections::HashSet;

use ratatui::buffer::Buffer;

use super::colors::effective;

const MINIMUM_TEXT_CONTRAST: f64 = 4.5;

pub(super) fn check(buffer: &Buffer) -> Vec<String> {
    let mut seen = HashSet::new();
    buffer
        .content
        .iter()
        .enumerate()
        .filter(|(_, cell)| meaningful(cell.symbol()))
        .filter_map(|(index, cell)| {
            let (foreground, background) = effective(cell);
            let ratio = contrast(foreground, background);
            if ratio >= MINIMUM_TEXT_CONTRAST || !seen.insert((foreground, background)) {
                return None;
            }
            let (x, y) = buffer.pos_of(index);
            Some(format!(
                "text {:?} at ({x},{y}): {foreground:?} on {background:?} has contrast {ratio:.6}:1; requires 4.5:1",
                cell.symbol()
            ))
        })
        .collect()
}

fn meaningful(symbol: &str) -> bool {
    symbol
        .chars()
        .any(|ch| !ch.is_whitespace() && !matches!(ch, '\u{2500}'..='\u{259f}'))
}

fn contrast(foreground: [u8; 3], background: [u8; 3]) -> f64 {
    let foreground = luminance(foreground);
    let background = luminance(background);
    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
}

fn luminance([red, green, blue]: [u8; 3]) -> f64 {
    0.2126_f64.mul_add(
        linear(red),
        0.7152_f64.mul_add(linear(green), 0.0722 * linear(blue)),
    )
}

fn linear(channel: u8) -> f64 {
    let value = f64::from(channel) / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests;
