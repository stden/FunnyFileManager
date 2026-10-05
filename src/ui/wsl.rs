//! WSL distributions, operation feedback and buttons from one shared layout.

mod geometry;

use crossterm::event::KeyCode;
use ratatui::{
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
    Frame,
};

use crate::models::{WslActivity, WslDialog, WslOperation};

use super::{far_block, head_cells, palette, Cells};
use geometry::{layout, Layout};

/// Return the key represented by the visible button under a click.
/// Rendering and mouse input use the same geometry, including narrow wrapping.
#[must_use]
pub fn wsl_button_at(screen: Rect, dialog: &WslDialog, column: u16, row: u16) -> Option<KeyCode> {
    layout(screen, dialog)
        .buttons
        .into_iter()
        .find_map(|button| {
            (button.control.enabled(dialog) && button.area.contains(Position::new(column, row)))
                .then_some(button.control.key)
        })
}

pub(super) fn render(f: &mut Frame<'_>, dialog: &WslDialog) {
    let layout = layout(f.area(), dialog);
    let ground = Style::default().bg(palette::MENU_BG).fg(palette::DRIVE_FG);
    let block = far_block()
        .title(Line::from(" WSL ").centered())
        .style(ground)
        .border_style(Style::default().fg(palette::DRIVE_FG));
    let inner = layout.inner;
    super::dialog::render_shadow(f, layout.area, f.area());
    f.render_widget(Clear, layout.area);
    f.render_widget(block, layout.area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    render_activity(f, dialog, &layout);
    let rows = distribution_rows(dialog, inner.width, layout.list_height, ground);
    f.render_widget(
        Paragraph::new(rows).style(ground),
        Rect {
            y: inner.y + layout.status_rows,
            height: layout.list_height,
            ..inner
        },
    );

    render_buttons(f, dialog, &layout, ground);
}

fn render_activity(f: &mut Frame<'_>, dialog: &WslDialog, layout: &Layout) {
    let inner = layout.inner;
    let (message, status_style) = activity(dialog);
    let message = if layout.status_rows == 1 {
        head_cells(&message, Cells::columns(inner.width))
    } else {
        message
    };
    f.render_widget(
        Paragraph::new(message)
            .style(status_style.bg(palette::MENU_BG))
            .wrap(Wrap { trim: true }),
        Rect {
            height: layout.status_rows,
            ..inner
        },
    );
}

fn distribution_rows(
    dialog: &WslDialog,
    width: u16,
    height: u16,
    ground: Style,
) -> Vec<Line<'static>> {
    if dialog.distributions().is_empty() {
        return vec![Line::from("No distributions")];
    }
    let start = dialog
        .selected()
        .unwrap_or(0)
        .saturating_sub(usize::from(height).saturating_sub(1));
    dialog
        .distributions()
        .iter()
        .enumerate()
        .skip(start)
        .take(usize::from(height))
        .map(|(index, distribution)| {
            let style = if dialog.selected() == Some(index) {
                Style::default()
                    .fg(palette::DRIVE_FG)
                    .bg(palette::MENU_SELECTED_BG)
                    .add_modifier(Modifier::BOLD)
            } else {
                ground
            };
            let text = head_cells(distribution.as_str(), Cells::columns(width));
            let padding = " ".repeat(usize::from(width).saturating_sub(Cells::of(&text).get()));
            Line::from(Span::styled(format!("{text}{padding}"), style))
        })
        .collect()
}

fn activity(dialog: &WslDialog) -> (String, Style) {
    let style = Style::default().fg(palette::DATA);
    match &dialog.activity {
        WslActivity::Ready => (
            "Select a distribution; Enter opens its root".to_owned(),
            style,
        ),
        WslActivity::Busy { operation, .. } => {
            let message = match operation {
                WslOperation::Refresh => "Refreshing distributions...".to_owned(),
                WslOperation::Update => "Updating WSL...".to_owned(),
                WslOperation::UpdatePackages(name) => {
                    format!("Updating packages: {}", name.as_str())
                }
                WslOperation::Open(name) => format!("Opening {}...", name.as_str()),
            };
            (message, style.fg(palette::MARKED))
        }
        WslActivity::Notice(message) => (message.clone(), style),
        WslActivity::Failed(message) => (format!("Error: {message}"), style.fg(Color::LightRed)),
    }
}

#[cfg(test)]
mod tests;

fn render_buttons(f: &mut Frame<'_>, dialog: &WslDialog, layout: &Layout, ground: Style) {
    for button in &layout.buttons {
        let text = head_cells(button.control.label, Cells::columns(button.area.width));
        let line = control_line(&text, button.control.enabled(dialog), ground);
        f.render_widget(Paragraph::new(line), button.area);
    }
}

fn control_line(text: &str, enabled: bool, ground: Style) -> Line<'_> {
    let normal = ground.fg(if enabled {
        palette::DRIVE_FG
    } else {
        palette::DISABLED_FG
    });
    let hotkey = if enabled {
        normal.fg(palette::HOTKEY)
    } else {
        normal
    };
    let (bracket, label) = text
        .strip_prefix('[')
        .map_or(("", text), |label| ("[", label));
    let (key, tail) = label
        .find(' ')
        .and_then(|space| label.split_at_checked(space))
        .unwrap_or((label, ""));
    Line::from(vec![
        Span::styled(bracket, normal),
        Span::styled(key, hotkey),
        Span::styled(tail, normal),
    ])
}
