//! Purpose: drawing Far's menu bar and its drop-downs.
//!
//! The bar is always on screen; the drop-down only while the menu is open. The
//! clock lives at the right end of the same row, because that is where Far
//! puts it and because the row is otherwise empty — a line of chrome that
//! carries one fact costs nothing extra.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::{far_block, palette, Cells};
use crate::menu::{Entry, Heading, Menu};
use crate::models::{App, Overlay};

/// Blank columns before the first heading and between two headings.
const GAP: &str = "   ";

/// Blank columns inside a drop-down, left of the tick and right of the
/// shortcut.
const PAD: usize = 1;

/// Blank columns the widest row keeps between its label and its shortcut; the
/// other rows' gaps are wider, since every shortcut is flush right.
const MIN_SHORTCUT_GAP: usize = 2;

/// Far's tick, against a checked row.
const TICK: char = '√';

/// The whole bar: the headings, and the clock at the far end.
pub(super) fn render_bar(f: &mut Frame, app: &App, area: Rect) {
    let ground = Style::default().bg(palette::MENU_BG).fg(palette::MENU_FG);
    // `Paragraph::new("")` restyles the row without writing a symbol to it, so
    // whatever was drawn underneath would keep showing through the gaps
    // between the headings. The bar owns its row.
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new("").style(ground), area);

    let open = match &app.overlay {
        Overlay::Menu(menu) => Some(menu.heading),
        _ => None,
    };

    let mut spans = Vec::new();
    for heading in Heading::ALL {
        spans.push(Span::styled(GAP, ground));
        // The open heading is picked out the way Far picks it out: reversed,
        // so the drop-down below it reads as hanging from it.
        let style = if open == Some(heading) {
            ground
                .bg(palette::MENU_SELECTED_BG)
                .fg(palette::MENU_SELECTED_FG)
        } else {
            ground
        };
        spans.extend(hotkey_spans(heading.label(), style));
    }
    f.render_widget(Paragraph::new(Line::from(spans)).style(ground), area);

    if app.settings.enabled(crate::menu::SettingsPage::Interface) {
        render_clock(f, ground, area);
    }
}

/// The time, right-aligned on the bar.
///
/// Read at draw time rather than kept in `App`: a clock is a fact about now,
/// and storing it would mean a second thing to keep in step with the truth.
fn render_clock(f: &mut Frame, ground: Style, area: Rect) {
    let text = crate::clock::now();
    let width = u16::try_from(Cells::of(&text).get()).unwrap_or(0);
    if width + 1 >= area.width {
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

/// The open heading's drop-down, hanging from the bar.
pub(super) fn render_drop_down(f: &mut Frame, app: &App, bar: Rect) {
    let Overlay::Menu(menu) = &app.overlay else {
        return;
    };
    let Some(area) = drop_down_area(menu, bar, f.area()) else {
        return;
    };

    let ground = Style::default()
        .bg(palette::MENU_BG)
        .fg(palette::MENU_SELECTED_FG);
    super::dialog::render_shadow(f, area, f.area());
    f.render_widget(Clear, area);
    let frame = far_block().border_style(ground).style(ground);
    let body = frame.inner(area);
    f.render_widget(frame, area);
    render_drop_down_entries(f, app, menu, ground, body);
}

fn render_drop_down_entries(f: &mut Frame, app: &App, menu: &Menu, ground: Style, body: Rect) {
    let first = menu
        .row
        .saturating_sub(usize::from(body.height.saturating_sub(1)));
    for (index, entry) in menu.entries().iter().enumerate().skip(first) {
        let Ok(offset) = u16::try_from(index - first) else {
            break;
        };
        if offset >= body.height {
            break;
        }
        let row = Rect {
            y: body.y + offset,
            height: 1,
            ..body
        };
        let entry = checked_entry(app, *entry);
        render_entry(f, &entry, index == menu.row, ground, row, &app.settings);
    }
}

/// `entry` with its tick set from the state it stands for.
pub fn checked_entry(app: &App, mut entry: Entry) -> Entry {
    let panel = app.get_active_panel_ref();
    let checked = if let Some(key) = entry.key() {
        match crate::action::resolve(crossterm::event::KeyEvent::new(key.code, key.modifiers)) {
            crate::action::Action::PanelView(mode) => panel.view == mode,
            _ => false,
        }
    } else {
        match entry.menu_command() {
            Some(crate::menu::MenuCommand::LongNames) => !panel.options.short_names,
            Some(crate::menu::MenuCommand::TogglePanel(side)) => app.panels_shown.shows(side),
            _ => false,
        }
    };
    if let Entry::Item { checked: mark, .. } = &mut entry {
        *mark = checked;
    }
    entry
}

/// One drop-down row.
fn render_entry(
    f: &mut Frame,
    entry: &Entry,
    selected: bool,
    ground: Style,
    area: Rect,
    settings: &crate::models::settings::Settings,
) {
    let Entry::Item {
        label,
        shortcut,
        checked,
        ..
    } = entry
    else {
        render_separator(f, ground, area);
        return;
    };

    let label = settings.translate(label);
    let style = entry_style(selected, entry.is_selectable(), ground);

    let mut spans = entry_label_spans(label, *checked, !selected, style);

    // The shortcut is pushed to the right edge, which is what lets the eye run
    // down the column of keys without reading the labels.
    let used = Cells::of(&label.replace('&', "")).get() + PAD + 2;
    let room = usize::from(area.width);
    let gap = room
        .saturating_sub(used)
        .saturating_sub(Cells::of(shortcut).get())
        .saturating_sub(PAD);
    spans.push(Span::styled(" ".repeat(gap), style));
    spans.push(Span::styled((*shortcut).to_string(), style));
    spans.push(Span::styled(" ".repeat(PAD), style));

    f.render_widget(Paragraph::new(Line::from(spans)).style(style), area);
}

fn entry_label_spans(label: &str, checked: bool, accent: bool, style: Style) -> Vec<Span<'static>> {
    let mut spans = vec![Span::styled(
        format!("{}{} ", " ".repeat(PAD), if checked { TICK } else { ' ' }),
        style,
    )];
    spans.extend(hotkey_spans_in(label, style, accent));
    spans
}

fn render_separator(f: &mut Frame, ground: Style, area: Rect) {
    // A rule, joined to the frame at both ends so it reads as part of it.
    let rule = format!("╟{}╢", "─".repeat(usize::from(area.width)));
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(rule, ground))).style(ground),
        Rect {
            x: area.x.saturating_sub(1),
            width: area.width + 2,
            ..area
        },
    );
}

/// Where the open drop-down sits: under its heading, clamped to the screen.
///
/// A pure function of the menu and the screen, so the mouse can ask the same
/// question the renderer answered.
pub(super) fn drop_down_area(menu: &Menu, bar: Rect, screen: Rect) -> Option<Rect> {
    let widest = menu_width(menu);
    let width = u16::try_from(widest.get())
        .unwrap_or(u16::MAX)
        .min(screen.width);
    let height = u16::try_from(menu.entries().len())
        .unwrap_or(u16::MAX)
        .saturating_add(2)
        .min(screen.height.saturating_sub(bar.height));
    if width < 4 || height < 3 {
        return None;
    }

    // Under its own heading, pulled left if it would run off the edge.
    let x = bar
        .x
        .saturating_add(heading_column(menu.heading))
        .min(screen.right().saturating_sub(width));
    Some(Rect {
        x,
        y: bar.y + bar.height,
        width,
        height,
    })
}

fn menu_width(menu: &Menu) -> Cells {
    menu.entries()
        .iter()
        .map(|entry| match entry {
            Entry::Item {
                label, shortcut, ..
            } => {
                Cells::of(&label.replace('&', ""))
                    + Cells::of(shortcut)
                    + Cells::new(PAD * 2 + 4 + MIN_SHORTCUT_GAP)
            }
            Entry::Separator => Cells::ZERO,
        })
        .max()
        .unwrap_or_default()
}

/// The column a heading starts at, counted the way [`render_bar`] draws them.
fn heading_column(wanted: Heading) -> u16 {
    let mut column = 0usize;
    for heading in Heading::ALL {
        column += Cells::of(GAP).get();
        if heading == wanted {
            break;
        }
        column += Cells::of(&heading.label().replace('&', "")).get();
    }
    u16::try_from(column).unwrap_or(0)
}

/// A label with its hotkey letter picked out.
fn hotkey_spans<'a>(label: &str, style: Style) -> Vec<Span<'a>> {
    hotkey_spans_in(label, style, true)
}

/// The same, with the accent suppressed where it would be a lie — a greyed
/// row's letter does nothing, and a selected row is already picked out.
fn hotkey_spans_in<'a>(label: &str, style: Style, accented: bool) -> Vec<Span<'a>> {
    let accent = if accented {
        style.fg(palette::HOTKEY).add_modifier(Modifier::BOLD)
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

/// The heading at `column` on the bar, if any.
///
/// The same arithmetic [`render_bar`] draws with, so a click lands on the
/// heading the user is pointing at.
#[must_use]
pub fn heading_at(bar: Rect, column: u16) -> Option<Heading> {
    let mut left = bar.x;
    for heading in Heading::ALL {
        left = left.saturating_add(u16::try_from(Cells::of(GAP).get()).unwrap_or(0));
        let width = u16::try_from(Cells::of(&heading.label().replace('&', "")).get()).unwrap_or(0);
        if (left..left.saturating_add(width)).contains(&column) {
            return Some(heading);
        }
        left = left.saturating_add(width);
    }
    None
}

/// The drop-down row at `(column, row)`, if the click is inside one.
#[must_use]
pub fn menu_row_at(menu: &Menu, screen: Rect, column: u16, row: u16) -> Option<usize> {
    let bar = Rect {
        height: 1,
        ..screen
    };
    let area = drop_down_area(menu, bar, screen)?;
    let body = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    if column < body.x || column >= body.x + body.width || row >= body.bottom() {
        return None;
    }
    let first = menu
        .row
        .saturating_sub(usize::from(body.height.saturating_sub(1)));
    let index = first + usize::from(row.checked_sub(body.y)?);
    menu.entries()
        .get(index)
        .filter(|entry| entry.is_selectable())
        .map(|_| index)
}

#[cfg(test)]
mod tests;

fn entry_style(selected: bool, enabled: bool, ground: Style) -> Style {
    if selected {
        ground
            .bg(palette::MENU_SELECTED_BG)
            .fg(palette::MENU_SELECTED_FG)
    } else if !enabled {
        // A command the program does not have yet, drawn but not offered.
        ground.fg(palette::DISABLED_FG)
    } else {
        ground
    }
}
