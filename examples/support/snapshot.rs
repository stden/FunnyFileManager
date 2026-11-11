//! Serialize cells captured from the real terminal renderer for visual review.

use std::{fmt::Write as _, fs, path::Path};

use ratatui::{
    buffer::{Buffer, Cell},
    style::Modifier,
};
use unicode_width::UnicodeWidthStr;

#[path = "colors.rs"]
pub mod colors;

fn hex([red, green, blue]: [u8; 3]) -> String {
    format!("#{red:02x}{green:02x}{blue:02x}")
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Save the terminal's exact glyphs and foreground/background colours as an SVG.
pub fn save(buffer: &Buffer, title: &str, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let width = u32::from(buffer.area.width) * 10;
    let height = u32::from(buffer.area.height) * 20;
    let title = escape(title);
    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"><title>{title}</title><g font-family=\"Consolas,DejaVu Sans Mono,monospace\" font-size=\"16\">");
    write_backgrounds(&mut svg, buffer)?;
    write_glyphs(&mut svg, buffer)?;
    svg.push_str("</g></svg>\n");
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, svg)?;
    Ok(())
}

fn write_backgrounds(svg: &mut String, buffer: &Buffer) -> std::fmt::Result {
    for y in 0..buffer.area.height {
        let mut next_column = 0;
        for x in 0..buffer.area.width {
            if x < next_column {
                continue;
            }
            let cell = &buffer[(buffer.area.x + x, buffer.area.y + y)];
            let remaining = buffer.area.width - x;
            let span = u16::try_from(UnicodeWidthStr::width(cell.symbol()).max(1))
                .unwrap_or(remaining)
                .min(remaining);
            next_column = x + span;
            write_background(svg, cell, x, y, span)?;
        }
    }
    Ok(())
}

fn write_background(svg: &mut String, cell: &Cell, x: u16, y: u16, span: u16) -> std::fmt::Result {
    let (_, background) = colors::effective(cell);
    write!(
        svg,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"20\" fill=\"{}\"/>",
        u32::from(x) * 10,
        u32::from(y) * 20,
        u32::from(span) * 10,
        hex(background)
    )
}

fn write_glyphs(svg: &mut String, buffer: &Buffer) -> std::fmt::Result {
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(buffer.area.x + x, buffer.area.y + y)];
            if cell.symbol().trim().is_empty() {
                continue;
            }
            let (foreground, _) = colors::effective(cell);
            let text = escape(cell.symbol());
            let attributes = text_attributes(cell.modifier);
            write!(
                svg,
                "<text x=\"{}\" y=\"{}\" fill=\"{}\"{attributes}>{text}</text>",
                u32::from(x) * 10,
                u32::from(y) * 20 + 16,
                hex(foreground)
            )?;
        }
    }
    Ok(())
}

fn text_attributes(modifier: Modifier) -> String {
    let mut attributes = String::new();
    if modifier.contains(Modifier::BOLD) {
        attributes.push_str(" font-weight=\"bold\"");
    }
    if modifier.contains(Modifier::ITALIC) {
        attributes.push_str(" font-style=\"italic\"");
    }
    let decoration = match (
        modifier.contains(Modifier::UNDERLINED),
        modifier.contains(Modifier::CROSSED_OUT),
    ) {
        (true, true) => "underline line-through",
        (true, false) => "underline",
        (false, true) => "line-through",
        (false, false) => "",
    };
    if !decoration.is_empty() {
        attributes.push_str(" text-decoration=\"");
        attributes.push_str(decoration);
        attributes.push('"');
    }
    attributes
}
