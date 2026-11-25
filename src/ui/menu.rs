//! Purpose: drawing Far's menu bar and its drop-downs.
//!
//! The bar is always on screen; the drop-down only while the menu is open. The
//! clock lives at the right end of the same row, because that is where Far
//! puts it and because the row is otherwise empty — a line of chrome that
//! carries one fact costs nothing extra.

mod drop_down;
mod geometry;

use crate::command::language::SettingsPage;
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::{palette, Cells};
use crate::menu::Heading;
use crate::models::{App, Overlay};
#[cfg(test)]
pub use drop_down::checked_entry;
pub(super) use drop_down::render_drop_down;
pub use geometry::{heading_at, menu_row_at};

/// Leading columns before the first heading on the menu bar.
const BAR_MARGIN: &str = "  ";

/// Blank columns before and after each heading label (included in its highlight).
const HEADING_PAD: &str = "  ";

/// Blank columns inside a drop-down, left of the tick and right of the
/// shortcut.
const PAD: usize = 1;

/// Blank columns the widest row keeps between its label and its shortcut; the
/// other rows' gaps are wider, since every shortcut is flush right.
const MIN_SHORTCUT_GAP: usize = 2;

/// Far's tick, against a checked row.
const TICK: char = '√';

/// The whole bar: the headings, and the clock at the far end.
pub(super) fn render_bar(f: &mut Frame<'_>, app: &App, area: Rect) {
    let ground = Style::default()
        .bg(palette::MENU_BAR_BG)
        .fg(palette::MENU_FG);
    // `Paragraph::new("")` restyles the row without writing a symbol to it, so
    // whatever was drawn underneath would keep showing through the gaps
    // between the headings. The bar owns its row.
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new("").style(ground), area);

    let open = match &app.overlay {
        Overlay::Menu(menu) => Some(menu.heading),
        _ => None,
    };

    let mut spans = vec![Span::styled(BAR_MARGIN, ground)];
    for heading in Heading::ALL {
        // The open heading is picked out the way Far picks it out: white on
        // black across both padding spaces, with its yellow hotkey intact.
        let style = if open == Some(heading) {
            Style::default()
                .bg(palette::MENU_SELECTED_BG)
                .fg(palette::MENU_SELECTED_FG)
        } else {
            ground
        };
        spans.push(Span::styled(HEADING_PAD, style));
        spans.extend(bar_hotkey_spans(heading.label(), style));
        spans.push(Span::styled(HEADING_PAD, style));
    }
    f.render_widget(Paragraph::new(Line::from(spans)).style(ground), area);

    if app.settings.enabled(SettingsPage::Interface) {
        render_clock(f, ground, area);
    }
}

/// The time, right-aligned on the bar.
///
/// Read at draw time rather than kept in `App`: a clock is a fact about now,
/// and storing it would mean a second thing to keep in step with the truth.
///
/// Only where it fits right of the headings, a blank apart: on a narrow
/// screen it was drawn over them (`Commands   Op 3:04 PMR`), hiding a
/// heading to show a time no one had asked for.
fn render_clock(f: &mut Frame<'_>, ground: Style, area: Rect) {
    let text = crate::clock::now();
    let width = u16::try_from(Cells::of(&text).get()).unwrap_or(u16::MAX);
    let needed = geometry::headings_width()
        .saturating_add(1)
        .saturating_add(width)
        .saturating_add(1);
    if needed > area.width {
        return;
    }
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(text, ground))).right_aligned(),
        Rect {
            x: area.right().saturating_sub(width + 1),
            width,
            ..area
        },
    );
}

/// A bar heading with its yellow hotkey letter (without underline).
fn bar_hotkey_spans<'a>(label: &str, style: Style) -> Vec<Span<'a>> {
    hotkey_spans_in(label, style, true)
}

/// A label with its hotkey letter picked out.
fn hotkey_spans_in<'a>(label: &str, style: Style, accented: bool) -> Vec<Span<'a>> {
    let accent = if accented {
        style.fg(palette::HOTKEY)
    } else {
        style
    };
    let Some((before, after)) = label.split_once('&') else {
        return vec![Span::styled(label.to_owned(), style)];
    };
    let mut rest = after.chars();
    let Some(letter) = rest.next() else {
        return vec![Span::styled(before.to_owned(), style)];
    };
    vec![
        Span::styled(before.to_owned(), style),
        Span::styled(letter.to_string(), accent),
        Span::styled(rest.as_str().to_owned(), style),
    ]
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_bar_colors;
#[cfg(test)]
mod tests_geometry;
