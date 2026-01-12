use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::models::{App, Panel, PanelSide};
#[cfg(test)]
use crate::models::Viewer;
use crate::utils::format_size;

pub const NAME_FIELD_WIDTH: usize = 30;

const FAR_BLUE: Color = Color::Rgb(0, 0, 128);

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(f.size());

    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    render_header(f, chunks[0]);

    render_panel(
        f,
        &mut app.left_panel,
        panels[0],
        app.active_panel == PanelSide::Left,
    );

    render_panel(
        f,
        &mut app.right_panel,
        panels[1],
        app.active_panel == PanelSide::Right,
    );

    render_status(f, app, chunks[2]);
    render_command_bar(f, chunks[3]);
    render_overlays(f, app);
}

fn render_panel(f: &mut Frame, panel: &mut Panel, area: Rect, is_active: bool) {
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Cyan)
    };

    let title = format!(" {} ", panel.path.display());

    let items: Vec<ListItem> = panel
        .entries
        .iter()
        .map(|entry| {
            let icon = if entry.is_dir { "📁" } else { "📄" };
            let size_str = if entry.is_dir {
                "<DIR>".to_string()
            } else {
                format_size(entry.size)
            };

            let line = Line::from(vec![
                Span::raw(icon),
                Span::raw(" "),
                Span::styled(
                    format!("{:<width$}", entry.name, width = NAME_FIELD_WIDTH),
                    if entry.is_dir {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Cyan)
                    },
                ),
                Span::styled(size_str, Style::default().fg(Color::LightCyan)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Style::default().bg(FAR_BLUE))
                .title(title)
                .title_style(Style::default().fg(Color::White).bg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("» ");

    f.render_stateful_widget(list, area, &mut panel.state);
}

fn render_header(f: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new("Far Manager style | Tab: Switch panels | F1 for help").style(
        Style::default()
            .fg(Color::Cyan)
            .bg(FAR_BLUE)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(paragraph, area);
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let panel = match app.active_panel {
        PanelSide::Left => &app.left_panel,
        PanelSide::Right => &app.right_panel,
    };

    let selected_info = if let Some(entry) = panel.get_selected_entry() {
        let size_info = if entry.is_dir {
            "DIR".to_string()
        } else {
            format_size(entry.size)
        };
        format!("{} | {}", entry.name, size_info)
    } else {
        "No selection".to_string()
    };

    let help_text = format!(
        " ↑↓: Navigate | Enter: Open | Tab: Switch Panel | ←→: Select Panel | {} | {}",
        selected_info, app.status_message
    );

    let paragraph = Paragraph::new(help_text)
        .style(Style::default().bg(FAR_BLUE).fg(Color::Cyan))
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(paragraph, area);
}

fn render_command_bar(f: &mut Frame, area: Rect) {
    let keys = [
        ("1", "Help"),
        ("2", "UserMn"),
        ("3", "View"),
        ("4", "Edit"),
        ("5", "Copy"),
        ("6", "RenMov"),
        ("7", "MkFold"),
        ("8", "Delete"),
        ("9", "ConfMn"),
        ("10", "Quit"),
    ];

    let spans: Vec<Span> = keys
        .iter()
        .flat_map(|(key, label)| {
            vec![
                Span::styled(
                    format!("{}", key),
                    Style::default().bg(Color::Black).fg(Color::Cyan),
                ),
                Span::styled(
                    format!("{:<6}", label),
                    Style::default().bg(Color::Cyan).fg(Color::Black),
                ),
            ]
        })
        .collect();

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Cyan).fg(Color::Black))
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(paragraph, area);
}

fn render_overlays(f: &mut Frame, app: &mut App) {
    if app.show_help {
        let area = centered_rect(80, 80, f.size());
        let help_lines = vec![
            "Far Manager style navigation:",
            "F1: Toggle this help window",
            "F3: View selected file (read-only)",
            "F4: Edit selected file with $EDITOR or nano",
            "F5: Copy selected entry to other panel",
            "F6: Move/Rename selected entry to other panel",
            "F7: Create a new directory in active panel",
            "F8: Delete selected entry",
            "F10 or Q: Quit",
            "Tab: switch active panel",
        ];

        let content = help_lines
            .into_iter()
            .map(|line| Line::from(Span::raw(line.to_string())))
            .collect::<Vec<Line>>();

        let block = Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(FAR_BLUE));

        let paragraph = Paragraph::new(content)
            .block(block)
            .style(Style::default().fg(Color::Cyan));

        f.render_widget(paragraph, area);
    }

    if let Some(viewer) = app.viewer.as_ref() {
        let area = centered_rect(90, 90, f.size());

        let block = Block::default()
            .title(format!(" View: {} (Esc to close) ", viewer.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(FAR_BLUE));

        let paragraph = Paragraph::new(
            viewer
                .lines
                .iter()
                .map(|line| Line::from(Span::raw(line.clone())))
                .collect::<Vec<Line>>(),
        )
        .scroll((viewer.scroll, 0))
        .block(block)
        .style(Style::default().fg(Color::Cyan));

        f.render_widget(paragraph, area);
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{App, DirEntry};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_ui_render_with_right_panel_active() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new().unwrap();
        app.active_panel = PanelSide::Right;

        terminal
            .draw(|f| {
                ui(f, &mut app);
            })
            .unwrap();
    }

    #[test]
    fn test_ui_render_status_with_file_selection() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new().unwrap();

        // Add a file to the panel
        app.left_panel.entries.push(DirEntry {
            name: "test.txt".to_string(),
            is_dir: false,
            size: 1024,
        });
        app.left_panel
            .state
            .select(Some(app.left_panel.entries.len() - 1));

        terminal
            .draw(|f| {
                ui(f, &mut app);
            })
            .unwrap();
    }

    #[test]
    fn test_ui_render_status_no_selection() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new().unwrap();

        // Clear panel selection
        app.left_panel.state.select(None);

        terminal
            .draw(|f| {
                ui(f, &mut app);
            })
            .unwrap();
    }

    #[test]
    fn test_ui_render_with_help_overlay() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new().unwrap();
        app.show_help = true;

        terminal
            .draw(|f| {
                ui(f, &mut app);
            })
            .unwrap();
    }

    #[test]
    fn test_ui_render_with_viewer_overlay() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new().unwrap();
        app.viewer = Some(crate::models::Viewer {
            title: "test.txt".to_string(),
            lines: vec!["Line 1".to_string(), "Line 2".to_string()],
            scroll: 0,
        });

        terminal
            .draw(|f| {
                ui(f, &mut app);
            })
            .unwrap();
    }

    #[test]
    fn test_render_status_right_panel_active_with_file() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new().unwrap();
        app.active_panel = PanelSide::Right;

        // Add a file to the right panel
        app.right_panel.entries.push(DirEntry {
            name: "document.pdf".to_string(),
            is_dir: false,
            size: 2048576, // 2MB
        });
        app.right_panel
            .state
            .select(Some(app.right_panel.entries.len() - 1));

        terminal
            .draw(|f| {
                ui(f, &mut app);
            })
            .unwrap();
    }

    #[test]
    fn test_render_panel_with_various_entries() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut app = App::new().unwrap();

        // Add various types of entries
        app.left_panel.entries.clear();
        app.left_panel.entries.push(DirEntry {
            name: "..".to_string(),
            is_dir: true,
            size: 0,
        });
        app.left_panel.entries.push(DirEntry {
            name: "documents".to_string(),
            is_dir: true,
            size: 0,
        });
        app.left_panel.entries.push(DirEntry {
            name: "image.jpg".to_string(),
            is_dir: false,
            size: 1536000, // ~1.5MB
        });

        terminal
            .draw(|f| {
                ui(f, &mut app);
            })
            .unwrap();
    }

    #[test]
    fn test_centered_rect_various_sizes() {
        let base = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };

        // Test different percentages
        let rect1 = centered_rect(80, 60, base);
        assert_eq!(rect1.width, 80);
        assert_eq!(rect1.height, 60);

        let rect2 = centered_rect(100, 100, base);
        assert_eq!(rect2.width, 100);
        assert_eq!(rect2.height, 100);

        let rect3 = centered_rect(10, 10, base);
        assert_eq!(rect3.width, 10);
        assert_eq!(rect3.height, 10);
    }

    #[test]
    fn test_ui_render_with_minimal_terminal_size() {
        let mut app = App::new().unwrap();
        app.left_panel.path = std::path::PathBuf::from("/test");
        app.right_panel.path = std::path::PathBuf::from("/test2");
        
        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        
        terminal.draw(|f| ui(f, &mut app)).unwrap();
    }

    #[test]
    fn test_ui_render_with_very_long_paths() {
        let mut app = App::new().unwrap();
        let long_path = "/very/long/path/that/exceeds/normal/display/width/in/terminal/window";
        app.left_panel.path = std::path::PathBuf::from(long_path);
        app.right_panel.path = std::path::PathBuf::from(long_path);
        app.status_message = "x".repeat(200);
        
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        
        terminal.draw(|f| ui(f, &mut app)).unwrap();
    }

    #[test]
    fn test_render_panel_with_empty_entries() {
        let mut app = App::new().unwrap();
        app.left_panel.entries.clear();
        app.left_panel.state.select(None);
        
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        
        terminal.draw(|f| {
            let area = f.size();
            render_panel(f, &mut app.left_panel, area, true);
        }).unwrap();
    }

    #[test]
    fn test_render_overlays_with_all_types() {
        let mut app = App::new().unwrap();
        
        app.show_help = true;
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
        app.show_help = false;
        
        app.viewer = Some(Viewer {
            lines: vec!["Line 1".to_string(), "Line 2".to_string()],
            scroll: 0,
            title: "Test Viewer".to_string(),
        });
        terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
        app.viewer = None;
        
        app.command_line = Some("test command".to_string());
        terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
        app.command_line = None;
        
        app.search = Some("search".to_string());
        terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
        app.search = None;
        
        app.show_bookmarks = true;
        app.bookmarks[0] = Some(std::path::PathBuf::from("/bookmark1"));
        app.bookmarks[5] = Some(std::path::PathBuf::from("/bookmark2"));
        terminal.draw(|f| render_overlays(f, &mut app)).unwrap();
    }

    #[test]
    fn test_render_status_with_extreme_values() {
        let mut app = App::new().unwrap();
        
        app.left_panel.entries = vec![
            DirEntry {
                name: "..".to_string(),
                is_dir: true,
                size: 0,
            },
            DirEntry {
                name: "huge_file.dat".to_string(),
                is_dir: false,
                size: u64::MAX,
            },
        ];
        app.left_panel.state.select(Some(1));
        
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        
        terminal.draw(|f| {
            let area = ratatui::layout::Rect::new(0, 0, 80, 1);
            render_status(f, &app, area);
        }).unwrap();
    }

    #[test]
    fn test_centered_rect_edge_cases() {
        let base = ratatui::layout::Rect::new(0, 0, 100, 50);
        
        let rect1 = centered_rect(0, 0, base);
        assert_eq!(rect1.width, 0);
        assert_eq!(rect1.height, 0);
        
        let rect2 = centered_rect(200, 100, base);
        assert_eq!(rect2.width, 100);
        assert_eq!(rect2.height, 50);
        
        let rect3 = centered_rect(1, 1, base);
        assert_eq!(rect3.width, 1);
        assert_eq!(rect3.height, 1);
        
        let tiny_base = ratatui::layout::Rect::new(0, 0, 1, 1);
        let rect4 = centered_rect(50, 50, tiny_base);
        assert_eq!(rect4.width, 1);
        assert_eq!(rect4.height, 1);
    }

    #[test]
    fn test_render_command_bar_variations() {
        let backend = TestBackend::new(120, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        
        terminal.draw(|f| {
            let area = f.size();
            render_command_bar(f, area);
        }).unwrap();
        
        let small_backend = TestBackend::new(40, 3);
        let mut small_terminal = Terminal::new(small_backend).unwrap();
        
        small_terminal.draw(|f| {
            let area = f.size();
            render_command_bar(f, area);
        }).unwrap();
    }

    #[test]
    fn test_render_panel_with_marked_files() {
        let mut app = App::new().unwrap();
        
        for i in 0..10 {
            app.left_panel.entries.push(DirEntry {
                name: format!("file{}.txt", i),
                is_dir: false,
                size: 100 * i,
            });
        }
        
        app.left_panel.marked.insert("file2.txt".to_string());
        app.left_panel.marked.insert("file5.txt".to_string());
        app.left_panel.marked.insert("file8.txt".to_string());
        app.left_panel.state.select(Some(5));
        
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        
        terminal.draw(|f| {
            let area = f.size();
            render_panel(f, &mut app.left_panel, area, true);
        }).unwrap();
    }
}
