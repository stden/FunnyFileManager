//! Serialize cells captured from the real terminal renderer for visual review.

use std::{fmt::Write as _, fs, path::Path};

use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier},
};

fn rgb(color: Color) -> String {
    let (red, green, blue) = match color {
        Color::Reset | Color::Gray => (192, 192, 192),
        Color::Black => (0, 0, 0),
        Color::Red => (128, 0, 0),
        Color::Green => (0, 128, 0),
        Color::Yellow => (128, 128, 0),
        Color::Blue => (0, 0, 128),
        Color::Magenta => (128, 0, 128),
        Color::Cyan => (0, 128, 128),
        Color::DarkGray => (128, 128, 128),
        Color::LightRed => (255, 0, 0),
        Color::LightGreen => (0, 255, 0),
        Color::LightYellow => (255, 255, 0),
        Color::LightBlue => (0, 0, 255),
        Color::LightMagenta => (255, 0, 255),
        Color::LightCyan => (0, 255, 255),
        Color::White => (255, 255, 255),
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(index) => return format!("var(--terminal-color-{index}, #c0c0c0)"),
    };
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
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            write_cell(
                &mut svg,
                &buffer[(buffer.area.x + x, buffer.area.y + y)],
                x,
                y,
            )?;
        }
    }
    svg.push_str("</g></svg>\n");
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, svg)?;
    Ok(())
}

fn write_cell(svg: &mut String, cell: &ratatui::buffer::Cell, x: u16, y: u16) -> std::fmt::Result {
    let (fg, bg) = if cell.modifier.contains(Modifier::REVERSED) {
        (cell.bg, cell.fg)
    } else {
        (cell.fg, cell.bg)
    };
    let text = escape(cell.symbol());
    write!(
        svg,
        "<rect x=\"{}\" y=\"{}\" width=\"10\" height=\"20\" fill=\"{}\"/>",
        u32::from(x) * 10,
        u32::from(y) * 20,
        rgb(bg)
    )?;
    if !text.trim().is_empty() {
        write!(
            svg,
            "<text x=\"{}\" y=\"{}\" fill=\"{}\">{text}</text>",
            u32::from(x) * 10,
            u32::from(y) * 20 + 16,
            rgb(fg)
        )?;
    }
    Ok(())
}
