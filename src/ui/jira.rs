//! Purpose: Render Jira issues browser, markdown detail viewer, and JQL editor dialog.

#[cfg(test)]
mod tests;

use ratatui::Frame;

use super::dialog::{self, Ground, Row};
use super::overlays::{clip_cells as clip, list_window as window, render_scrolled_lines};
use crate::models::services::jira::{JiraDialog, JiraMode};
use crate::mouse::DialogMouse;

pub(super) fn render(f: &mut Frame<'_>, tracker: &mut DialogMouse, dialog: &JiraDialog) {
    let max_w = usize::from(f.area().width.saturating_sub(12)).clamp(32, 96);
    let max_h = usize::from(f.area().height.saturating_sub(14)).clamp(4, 14);
    let caret = match &dialog.mode {
        JiraMode::Browse => render_browse(f, tracker, dialog, max_w, max_h),
        JiraMode::Detail {
            title,
            lines,
            scroll,
        } => render_scrolled_lines(
            f,
            tracker,
            title,
            lines,
            *scroll,
            (
                "(empty issue details)",
                "Up/Down/PgUp/PgDn Scroll | Enter/F3/Esc Back",
            ),
            (max_w, max_h),
        ),
        JiraMode::EditJql(editor) => render_jql(f, tracker, dialog, editor, max_w),
    };
    if let Some(pos) = caret {
        f.set_cursor_position(pos);
    }
}

fn render_browse(
    f: &mut Frame<'_>,
    tracker: &mut DialogMouse,
    dialog: &JiraDialog,
    max_w: usize,
    max_h: usize,
) -> Option<(u16, u16)> {
    let filter_label = if dialog.filter.is_empty() {
        "-"
    } else {
        &dialog.filter
    };
    let header = clip(
        &format!(
            "JQL: {} | Filter: {filter_label} | Marks: {}",
            dialog.jql,
            dialog.marks.len()
        ),
        max_w,
    );
    let lines = browse_lines(dialog, max_w, max_h);
    let status = clip(&dialog.status, max_w);
    let keys = clip(
        "Enter/F3 View  Space Mark  F2 JQL  F5/Shift Export  F6 All  F7 Snap  Ctrl+R Reload",
        max_w,
    );
    let mut rows = vec![Row::Plain(&header), Row::Divider];
    rows.extend(lines.iter().map(|line| Row::Plain(line)));
    rows.extend([Row::Divider, Row::Plain(&status), Row::Plain(&keys)]);
    dialog::render_tracked(f, tracker, "Jira Issues", Ground::Prompt, &rows, &[])
}

fn browse_lines(dialog: &JiraDialog, max_w: usize, max_h: usize) -> Vec<String> {
    let visible = dialog.filtered_indices();
    if visible.is_empty() {
        return empty_browse_lines(dialog, max_w);
    }
    let range = window(visible.len(), dialog.cursor, max_h);
    range
        .filter_map(|pos| {
            visible
                .get(pos)
                .and_then(|&idx| dialog.issues.get(idx).map(|i| (pos, i)))
        })
        .map(|(pos, issue)| {
            let cur = if pos == dialog.cursor { ">" } else { " " };
            let mark = if dialog.marks.contains(&issue.key) {
                "[x]"
            } else {
                "[ ]"
            };
            clip(
                &format!(
                    "{cur} {mark} {:<10} [{}] {}",
                    issue.key, issue.status, issue.summary
                ),
                max_w,
            )
        })
        .collect()
}

fn empty_browse_lines(dialog: &JiraDialog, max_w: usize) -> Vec<String> {
    if let Some(err) = &dialog.setup_error {
        return vec![
            clip(err, max_w),
            clip(
                "Set JIRA_PERSONAL_TOKEN / JIRA_URL or press F4 to edit ffm.yml",
                max_w,
            ),
        ];
    }
    vec!["No Jira issues loaded — press F2 for JQL or Ctrl+R to reload".to_owned()]
}

fn render_jql(
    f: &mut Frame<'_>,
    tracker: &mut DialogMouse,
    dialog: &JiraDialog,
    editor: &crate::lineedit::LineEditor,
    max_w: usize,
) -> Option<(u16, u16)> {
    let status = clip(&dialog.status, max_w);
    let rows = [
        Row::Label("JQL query (Enter search, Esc back):"),
        Row::line(editor, true, false),
        Row::Divider,
        Row::Plain(&status),
    ];
    dialog::render_tracked(f, tracker, "Edit Jira JQL", Ground::Prompt, &rows, &[])
}
