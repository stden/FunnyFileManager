//! Purpose: A panel's info row above the bottom border: the current entry's
//! full name and the columns its view leaves out, or the message or quick
//! search that takes the row's place.

use std::borrow::Cow;

use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::{fit_quoting_path, head_cells, Cells, FAR_BLUE, PANEL_INFO_ROW};
use super::columns::PanelColumn;
use super::fields::pad_cells;
use super::layout::MIN_NAME_WIDTH;
use crate::models::{App, DirEntry, Panel};
use crate::status::PathMessage;

/// The row above the bottom border, which Far gives to the current entry's
/// full name — the one place a name too long for its column can still be read.
///
/// It doubles as this UI's message line. Far has no status bar to lose, so
/// removing ours left `status_text` (the only channel a failed copy or a
/// refused view has) with nowhere to go; it takes precedence here, and quick
/// search takes precedence over it, because both are about what the user is
/// doing right now while the name merely repeats the highlighted row.
///
/// A view whose table leaves out some of Size, Date and Time adds the current
/// entry's after the name (see [`info_columns`]), as Far's status line does.
pub(super) fn render_panel_info(
    f: &mut Frame<'_>,
    panel: &Panel,
    inner: Rect,
    footer: Option<&PathMessage<'_>>,
    details: &[PanelColumn],
) {
    let width = Cells::columns(inner.width);
    let text = match (footer, panel.get_selected_entry()) {
        (Some(message), _) => {
            fit_quoting_path(&message.before, &message.path, &message.after, width)
        }
        (None, Some(entry)) => info_text(entry, details, width),
        (None, None) => String::new(),
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(Color::LightCyan).bg(FAR_BLUE),
        ))),
        Rect {
            y: inner.y + inner.height - PANEL_INFO_ROW,
            height: PANEL_INFO_ROW,
            ..inner
        },
    );
}

/// The current entry's name, then its `details` right-aligned — what a view
/// with no room for those columns shows of them.
///
/// The name is what the row is for, so where both do not fit the details
/// give way rather than the name falling under [`MIN_NAME_WIDTH`].
fn info_text(entry: &DirEntry, details: &[PanelColumn], width: Cells) -> String {
    let shown = crate::ui::printable_name(&entry.name);
    let mut columns = String::new();
    for column in details {
        columns.push(' ');
        columns.push_str(&column.lay_out(&column.cell(entry)));
    }
    let name = width.saturating_sub(Cells::of(&columns));
    if columns.is_empty() || name < MIN_NAME_WIDTH {
        return head_cells(&shown, width);
    }
    format!("{}{columns}", pad_cells(&head_cells(&shown, name), name))
}

/// What the active panel's info row says, when it is not just the entry name:
/// the message, split around the path it quotes so a narrow row can shorten
/// the path rather than lose the file name or the reason.
pub(in crate::ui) fn panel_footer(app: &App) -> Option<PathMessage<'static>> {
    if let Some(search) = app.quick_search.query() {
        return Some(PathMessage::from(format!("Search: {search}_")));
    }
    let news = app.status_news()?;
    // Status lines quote file names — "Viewing x", "Saved x", a failed copy's
    // path — so the message gets the same treatment the name it replaces does.
    let printable = |text: &str| -> Cow<'static, str> {
        Cow::Owned(crate::ui::printable_name(text).into_owned())
    };
    Some(app.status.path_message().map_or_else(
        || PathMessage::from(crate::ui::printable_name(&news).into_owned()),
        |message| PathMessage {
            before: printable(&message.before),
            path: printable(&message.path),
            after: printable(&message.after),
        },
    ))
}
