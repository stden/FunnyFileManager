//! Purpose: The open menu's drop-down: its frame and shadow under the bar,
//! one row per entry with the tick, the hotkey letter and the shortcut flush
//! right, rules for separators, and each row's colours.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::super::{far_block, palette, Cells};
use super::geometry::drop_down_area;
use super::{hotkey_spans_in, PAD, TICK};
use crate::menu::{Entry, Menu};
use crate::models::{App, Overlay};

/// The open heading's drop-down, hanging from the bar.
pub(in crate::ui) fn render_drop_down(f: &mut Frame<'_>, app: &App, bar: Rect) {
    let Overlay::Menu(menu) = &app.overlay else {
        return;
    };
    let Some(area) = drop_down_area(menu, bar, f.area()) else {
        return;
    };

    let ground = Style::default()
        .bg(palette::MENU_BG)
        .fg(palette::MENU_SELECTED_FG);
    super::super::dialog::render_shadow(f, area, f.area());
    f.render_widget(Clear, area);
    let frame = far_block().border_style(ground).style(ground);
    let body = frame.inner(area);
    f.render_widget(frame, area);
    render_drop_down_entries(f, app, menu, ground, body);
}

fn render_drop_down_entries(f: &mut Frame<'_>, app: &App, menu: &Menu, ground: Style, body: Rect) {
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
    use crate::action::{Action, PanelCommand};
    let panel = app.get_active_panel_ref();
    let checked = match entry.action() {
        Some(Action::PanelView(mode)) => panel.view == mode,
        Some(Action::Panel(PanelCommand::LongNames)) => !panel.options.short_names,
        Some(Action::Panel(PanelCommand::TogglePanel(side))) => app.panels_shown.shows(side),
        _ => false,
    };
    if let Entry::Item { checked: mark, .. } = &mut entry {
        *mark = checked;
    }
    entry
}

/// One drop-down row.
fn render_entry(
    f: &mut Frame<'_>,
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
    spans.push(Span::styled((*shortcut).to_owned(), style));
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

fn render_separator(f: &mut Frame<'_>, ground: Style, area: Rect) {
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

#[cfg(test)]
mod tests_disabled;
