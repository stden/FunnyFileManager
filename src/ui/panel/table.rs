//! Purpose: The table's chrome inside a panel: the column-title row under the
//! top border, with the sort letter in its corner, and the divider rule that
//! closes the column separators above the info row.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::{palette, Cells, FAR_BLUE};
use super::columns::COLUMN_SEPARATOR;
use super::fields::center_cells;
use super::layout::PanelLayout;
use crate::models::SortMode;

/// The column-title row Far draws directly under the top border.
///
/// The leading letter is the sort mode — `n`ame, `e`xtension, `s`ize and the
/// rest of [`SortMode::letter`] — in the corner Far puts it, which is the only
/// place the panel says how it is ordered.
pub(super) fn header_line(sort_mode: SortMode, layout: PanelLayout) -> Line<'static> {
    // Bright white, like the values under it — Far reserves cyan for the frame
    // and the rules, not for text.
    let style = Style::default().fg(palette::DATA).bg(FAR_BLUE);

    let mut spans = Vec::new();
    for stripe in 0..layout.stripes {
        let name = layout.name_width(stripe);
        let title = if stripe == 0 {
            // Only the first title gives up a cell to the letter.
            let title = center_cells("Name", name.saturating_sub(Cells(1)));
            format!("{}{title}", sort_mode.letter())
        } else {
            spans.push(Span::styled(COLUMN_SEPARATOR, style));
            center_cells("Name", name)
        };
        spans.push(Span::styled(title, style));
        for column in layout.columns {
            spans.push(Span::styled(COLUMN_SEPARATOR, style));
            spans.push(Span::styled(
                center_cells(column.title(), column.width()),
                style,
            ));
        }
    }
    Line::from(spans)
}

/// The rule that closes the column separators above the info row.
///
/// Far does not let the column lines run into the frame: they stop on a
/// horizontal divider whose `┴` junctions say which column each one belonged
/// to. It spans the whole panel width, border columns included, so the two ends
/// meet the double frame as `╟` and `╢` rather than leaving a gap.
fn column_rule(layout: PanelLayout) -> String {
    let mut rule = String::from("╟");
    for stripe in 0..layout.stripes {
        if stripe > 0 {
            rule.push('┴');
        }
        rule.push_str(&"─".repeat(layout.name_width(stripe).get()));
        for column in layout.columns {
            rule.push('┴');
            rule.push_str(&"─".repeat(column.width().get()));
        }
    }
    rule.push('╢');
    rule
}

/// The column rule on the `divider` row of a panel drawn in `area`.
pub(super) fn render_column_rule(
    f: &mut Frame<'_>,
    layout: PanelLayout,
    area: Rect,
    divider: Rect,
    style: Style,
) {
    // The rule spans `area`, not `inner`: its two ends are the frame cells the
    // block already drew, and overwriting them is what turns a floating dash
    // into a divider joined to the border.
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(column_rule(layout), style)))
            .style(Style::default().bg(FAR_BLUE)),
        Rect {
            x: area.x,
            width: area.width,
            ..divider
        },
    );
}
