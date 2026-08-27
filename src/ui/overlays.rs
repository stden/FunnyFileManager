//! Modal overlay rendering: help, viewer, built-in editor, dialogs.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{cells_before, centered_rect, skip_cells, take_cells, Cells, FAR_BLUE};
use crate::models::{App, Overlay};

pub fn render_overlays(f: &mut Frame, app: &App) {
    match &app.overlay {
        // Nothing to draw on top: the command line is part of the main layout,
        // not a floating window.
        Overlay::None | Overlay::CommandLine(_) => {}
        Overlay::Help => {
            let area = centered_rect(80, 80, f.area());
            let help_lines = vec![
                "Far Manager style navigation:",
                "F1: Toggle this help window",
                "F3: View selected file (read-only)",
                "F4: Edit selected file (built-in editor, F2 saves)",
                "Alt+F4: Edit selected file with $EDITOR or VS Code",
                "F5: Copy selected entry to other panel",
                "F6: Move/Rename selected entry to other panel",
                "F7: Create a new directory in active panel",
                "F8: Delete selected entry",
                "F9: MCP Server status (AI tools)",
                "F10: Quit (or type `quit`)",
                "Tab: switch active panel",
                "PgUp/PgDn: move the selection one screen up/down",
                ":  - command line (Tab completes verbs, args & cd paths)",
                ":mcp - MCP server status & installation",
                ":winmsg - recent Windows system warnings/errors",
                ":winperf - Windows performance summary",
            ];

            let content = help_lines
                .into_iter()
                .map(|line| Line::from(Span::raw(line)))
                .collect::<Vec<Line>>();

            let block = Block::default()
                .title(" Help ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .style(Style::default().bg(FAR_BLUE));

            let paragraph = Paragraph::new(content)
                .block(block)
                .style(Style::default().fg(Color::Cyan));

            f.render_widget(Clear, area);
            f.render_widget(paragraph, area);
        }
        Overlay::Viewer(viewer) => {
            let area = centered_rect(90, 90, f.area());
            let inner_h = usize::from(area.height.saturating_sub(2));

            let block = Block::default()
                .title(format!(" View: {} (Esc to close) ", viewer.title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .style(Style::default().bg(FAR_BLUE));

            // Slice the visible window here instead of via Paragraph::scroll:
            // that takes u16, so an offset past 65535 would wrap around and show
            // the wrong part of the file. Slicing also keeps the built Vec small.
            let body: Vec<Line> = viewer
                .lines
                .iter()
                .skip(viewer.scroll.lines())
                .take(inner_h)
                .map(|line| Line::from(Span::raw(line.as_str())))
                .collect();

            let paragraph = Paragraph::new(body)
                .block(block)
                .style(Style::default().fg(Color::Cyan));

            f.render_widget(Clear, area);
            f.render_widget(paragraph, area);
        }
        Overlay::Editor(editor) => {
            let area = centered_rect(90, 90, f.area());
            let inner_h = usize::from(area.height.saturating_sub(2));
            let inner_w = Cells::columns(area.width.saturating_sub(2));

            // Stateless scrolling: keep the cursor roughly centered vertically
            // and always visible horizontally — no scroll state to desync.
            let scroll_y = editor.cursor_line.saturating_sub(inner_h / 2);

            // `cursor_col` is a CHAR index into the buffer, which is what
            // editing needs — but the screen budgets in CELLS, and a CJK glyph
            // spends two. Scrolling and positioning by the char index put both
            // the window and the cursor in the wrong column on any non-ASCII
            // line; convert once, here, and work in cells below.
            let cursor_line_text = editor
                .lines
                .get(editor.cursor_line)
                .map_or("", String::as_str);
            let cursor_cell = cells_before(cursor_line_text, editor.cursor_column());
            let scroll_x = cursor_cell.saturating_sub(inner_w.saturating_sub(Cells::new(1)));

            let modified_marker = if editor.state.is_modified() { " *" } else { "" };
            let block = Block::default()
                .title(format!(
                    " Edit: {}{} (F2 Save, Esc Close) ",
                    editor.title, modified_marker
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .style(Style::default().bg(FAR_BLUE));

            // Slice the visible window here instead of via Paragraph::scroll:
            // that takes u16, so an offset past 65535 (one-line minified files)
            // would wrap around and draw text the cursor is not on, while the
            // cursor below uses the untruncated offset. Cells, never bytes and
            // never chars.
            let body: Vec<Line> = editor
                .lines
                .iter()
                .skip(scroll_y)
                .take(inner_h)
                .map(|line| Line::from(take_cells(skip_cells(line, scroll_x).0, inner_w).0))
                .collect();

            let paragraph = Paragraph::new(body)
                .block(block)
                .style(Style::default().fg(Color::White));

            f.render_widget(Clear, area);
            f.render_widget(paragraph, area);

            if editor.state.is_confirming_exit() {
                let dialog = centered_rect(50, 20, f.area());
                let content = vec![
                    Line::from(Span::styled(
                        format!("Save changes to {}?", editor.title),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "[Y]es  [N]o  [Esc] Continue editing",
                        Style::default().fg(Color::Cyan),
                    )),
                ];
                let dialog_block = Block::default()
                    .title(" Unsaved Changes ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .style(Style::default().bg(FAR_BLUE));
                f.render_widget(Clear, dialog);
                f.render_widget(Paragraph::new(content).block(dialog_block), dialog);
            } else {
                // Position against the cells actually dropped, not against
                // `scroll_x`: a double-width glyph straddling the cut is
                // dropped whole, so the two can differ by one.
                let dropped = skip_cells(cursor_line_text, scroll_x).1;
                let cursor_x = area.x + 1 + cursor_cell.saturating_sub(dropped).get() as u16;
                let cursor_y = area.y + 1 + (editor.cursor_line - scroll_y) as u16;
                f.set_cursor_position((cursor_x, cursor_y));
            }
        }
        Overlay::DriveSelection {
            side: target_panel,
            selected_index,
        } => {
            let area = centered_rect(50, 60, f.area());
            let drives = crate::models::App::get_all_drives();

            let title = match target_panel {
                crate::models::PanelSide::Left => " Select Drive (Left Panel) ",
                crate::models::PanelSide::Right => " Select Drive (Right Panel) ",
            };

            let drive_lines: Vec<Line> = drives
                .iter()
                .enumerate()
                .map(|(i, d)| {
                    let prefix = if i == *selected_index { "► " } else { "  " };
                    let style = if i == *selected_index {
                        Style::default().fg(Color::Black).bg(Color::Cyan)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    Line::from(Span::styled(format!("{}{}", prefix, d.display()), style))
                })
                .collect();

            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow))
                .style(Style::default().bg(FAR_BLUE));

            let paragraph = Paragraph::new(drive_lines)
                .block(block)
                .style(Style::default().fg(Color::White));

            f.render_widget(Clear, area);
            f.render_widget(paragraph, area);
        }
        Overlay::DeleteConfirm(files) => {
            let area = centered_rect(60, 40, f.area());

            let file_count = files.len();
            let preview: Vec<Line> = files
                .iter()
                .take(5)
                .map(|p| {
                    let name = p.file_name().map_or_else(
                        || p.display().to_string(),
                        |n| n.to_string_lossy().into_owned(),
                    );
                    Line::from(Span::raw(format!("  {name}")))
                })
                .collect();

            let mut content = vec![
                Line::from(Span::styled(
                    format!("Delete {file_count} item(s)?"),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
            ];
            content.extend(preview);
            if file_count > 5 {
                content.push(Line::from(Span::raw(format!(
                    "  ... and {} more",
                    file_count - 5
                ))));
            }
            content.push(Line::from(""));
            content.push(Line::from(Span::styled(
                "[Y]es  [N]o  [Esc] Cancel",
                Style::default().fg(Color::Cyan),
            )));

            let block = Block::default()
                .title(" Confirm Delete ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .style(Style::default().bg(FAR_BLUE));

            let paragraph = Paragraph::new(content)
                .block(block)
                .style(Style::default().fg(Color::White));

            f.render_widget(Clear, area);
            f.render_widget(paragraph, area);
        }
        Overlay::NewFileInput(input) => {
            let area = centered_rect(50, 20, f.area());

            let block = Block::default()
                .title(" Create New File (Shift+F4) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .style(Style::default().bg(FAR_BLUE));

            let content = vec![
                Line::from(Span::raw("Enter filename:")),
                Line::from(Span::styled(
                    format!("{input}_"),
                    Style::default().fg(Color::White).bg(Color::DarkGray),
                )),
            ];

            let paragraph = Paragraph::new(content)
                .block(block)
                .style(Style::default().fg(Color::Cyan));

            f.render_widget(Clear, area);
            f.render_widget(paragraph, area);
        }
    }
}
