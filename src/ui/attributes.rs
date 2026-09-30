//! Purpose: Draws the File attributes dialog (Ctrl+A).
//!
//! Far lays the permission boxes out three to a row — Owner, Group, Other —
//! and the dialog's row-per-control frame has no such row. So each group is
//! drawn as one caption line, `Owner   [x] Read  [ ] Write  [x] Execute`,
//! and the box under the cursor is painted over afterwards in the focus
//! colour. Twelve boxes stacked one per line would not fit a 24-row screen
//! with the times and buttons under them.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::Paragraph,
    Frame,
};

use super::{dialog, palette};
use crate::models::{AttributesDialog, AttributesField, Flag};

const TITLE: &str = "File attributes";

/// Columns a group's caption takes before its first box.
const CAPTION_WIDTH: usize = 8;
/// Blanks between one box's label and the next box.
const BOX_GAP: &str = "  ";
/// Rows above the first group line: the selection, the names, a rule.
const ROWS_ABOVE_GROUPS: usize = 3;

/// One drawn row of boxes: its text, and where each `[` sits in it.
struct GroupLine {
    text: String,
    box_columns: Vec<usize>,
}

pub(super) fn render(
    f: &mut Frame<'_>,
    tracker: &mut crate::mouse::DialogMouse,
    dialog: &AttributesDialog,
) {
    let headline = dialog.headline();
    let names = format!("Owner: {}   Group: {}", dialog.owner, dialog.group);
    let groups = group_lines(dialog);
    let rows = rows(dialog, &headline, &names, &groups);
    let on = |field| dialog.field == field;
    let buttons = [
        dialog::Button::new("&Set")
            .default_button()
            .focused(on(AttributesField::Set)),
        dialog::Button::new("&Cancel").focused(on(AttributesField::Cancel)),
    ];
    let caret = dialog::render_tracked(f, tracker, TITLE, dialog::Ground::Prompt, &rows, &buttons);
    if let (AttributesField::Flag(index), Some(body)) = (dialog.field, tracker.body) {
        highlight_box(f, dialog, &groups, index, body);
    }
    if let Some(caret) = caret {
        f.set_cursor_position(caret);
    }
}

/// Every group as one line, boxes in [`Flag::GROUPS`] order.
fn group_lines(dialog: &AttributesDialog) -> Vec<GroupLine> {
    let mut boxes = dialog.flags.iter();
    Flag::GROUPS
        .iter()
        .map(|group| {
            let mut text = format!("{:<CAPTION_WIDTH$}", group.caption);
            let mut box_columns = Vec::with_capacity(group.flags.len());
            for flag in group.flags {
                let mark = boxes.next().map_or(' ', |flag_box| flag_box.current.mark());
                box_columns.push(text.chars().count());
                text = format!("{text}[{mark}] {}{BOX_GAP}", flag.label());
            }
            GroupLine { text, box_columns }
        })
        .collect()
}

fn rows<'a>(
    dialog: &'a AttributesDialog,
    headline: &'a str,
    names: &'a str,
    groups: &'a [GroupLine],
) -> Vec<dialog::Row<'a>> {
    let on = |field| dialog.field == field;
    let mut rows = vec![
        dialog::Row::Label(headline),
        dialog::Row::Label(names),
        dialog::Row::Divider,
    ];
    rows.extend(groups.iter().map(|group| dialog::Row::Label(&group.text)));
    rows.extend([
        dialog::Row::Divider,
        dialog::Row::text_field(
            "&Modification:",
            &dialog.modified.current,
            on(AttributesField::Modified),
            true,
        ),
        dialog::Row::text_field(
            "&Access time:",
            &dialog.accessed.current,
            on(AttributesField::Accessed),
            true,
        ),
        dialog::Row::Divider,
        dialog::Row::Checkbox {
            label: "Process &subfolders",
            checked: dialog.process_subfolders,
            focused: on(AttributesField::Subfolders),
            enabled: dialog.has_directories,
        },
    ]);
    rows
}

/// Paint the `index`-th box again in the focus colour, where the frame drew
/// it. The frame's own checkbox row does this for one box per line; here a
/// line holds three, so the highlight is placed by hand.
fn highlight_box(
    f: &mut Frame<'_>,
    dialog: &AttributesDialog,
    groups: &[GroupLine],
    index: usize,
    body: Rect,
) {
    let Some(flag_box) = dialog.flags.get(index) else {
        return;
    };
    let Some((row, column)) = locate_box(groups, index) else {
        return;
    };
    let (Ok(row), Ok(column)) = (u16::try_from(row), u16::try_from(column)) else {
        return;
    };
    let area = Rect {
        x: body.x.saturating_add(column),
        y: body.y.saturating_add(row),
        width: 3,
        height: 1,
    };
    if area.right() > body.right() || area.y >= body.bottom() {
        return;
    }
    let focus = Style::default()
        .bg(palette::DIALOG_BG)
        .fg(palette::DIALOG_FG)
        .add_modifier(Modifier::REVERSED);
    let mark = format!("[{}]", flag_box.current.mark());
    f.render_widget(Paragraph::new(Span::styled(mark, focus)), area);
}

/// The body row and column of the `index`-th box across all groups.
fn locate_box(groups: &[GroupLine], index: usize) -> Option<(usize, usize)> {
    groups
        .iter()
        .enumerate()
        .flat_map(|(line, group)| {
            group
                .box_columns
                .iter()
                .map(move |column| (ROWS_ABOVE_GROUPS + line, *column))
        })
        .nth(index)
}

#[cfg(test)]
mod tests;
