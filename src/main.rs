use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::{
    cmp::Ordering,
    error::Error,
    fs, io,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

const NAME_FIELD_WIDTH: usize = 30;

#[derive(Debug)]
struct Panel {
    path: PathBuf,
    entries: Vec<DirEntry>,
    state: ListState,
}

#[derive(Debug, Clone)]
struct DirEntry {
    name: String,
    is_dir: bool,
    size: u64,
}

#[derive(Default)]
struct Viewer {
    title: String,
    lines: Vec<String>,
    scroll: u16,
}

impl Panel {
    fn new(path: PathBuf) -> io::Result<Self> {
        let mut panel = Panel {
            path,
            entries: Vec::new(),
            state: ListState::default(),
        };
        panel.load_entries()?;
        if !panel.entries.is_empty() {
            panel.state.select(Some(0));
        }
        Ok(panel)
    }

    fn load_entries(&mut self) -> io::Result<()> {
        self.entries.clear();

        // Add parent directory entry
        if self.path.parent().is_some() {
            self.entries.push(DirEntry {
                name: "..".to_string(),
                is_dir: true,
                size: 0,
            });
        }

        let mut entries: Vec<DirEntry> = fs::read_dir(&self.path)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let metadata = entry.metadata().ok()?;
                let name = entry.file_name().to_string_lossy().to_string();

                Some(DirEntry {
                    name,
                    is_dir: metadata.is_dir(),
                    size: metadata.len(),
                })
            })
            .collect();

        // Sort: directories first, then files
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        self.entries.extend(entries);
        Ok(())
    }

    fn next(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.entries.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.entries.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn enter(&mut self) -> io::Result<()> {
        if let Some(i) = self.state.selected() {
            if let Some(entry) = self.entries.get(i) {
                if entry.is_dir {
                    if entry.name == ".." {
                        if let Some(parent) = self.path.parent() {
                            self.path = parent.to_path_buf();
                        }
                    } else {
                        self.path = self.path.join(&entry.name);
                    }
                    self.load_entries()?;
                    if !self.entries.is_empty() {
                        self.state.select(Some(0));
                    }
                }
            }
        }
        Ok(())
    }

    fn get_selected_entry(&self) -> Option<&DirEntry> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    fn ensure_selection(&mut self) {
        if !self.entries.is_empty() && self.state.selected().is_none() {
            self.state.select(Some(0));
        }
    }

    fn selected_path(&self) -> Option<PathBuf> {
        self.get_selected_entry()
            .map(|entry| self.path.join(&entry.name))
    }
}

struct App {
    left_panel: Panel,
    right_panel: Panel,
    active_panel: PanelSide,
    show_help: bool,
    viewer: Option<Viewer>,
    status_message: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PanelSide {
    Left,
    Right,
}

impl App {
    fn new() -> io::Result<Self> {
        let current_dir = std::env::current_dir()?;
        Ok(App {
            left_panel: Panel::new(current_dir.clone())?,
            right_panel: Panel::new(current_dir)?,
            active_panel: PanelSide::Left,
            show_help: false,
            viewer: None,
            status_message: String::from(
                "F1 Help  F3 View  F4 Edit  F5 Copy  F6 Move  F7 Mkdir  F8 Delete",
            ),
        })
    }

    fn get_active_panel(&mut self) -> &mut Panel {
        match self.active_panel {
            PanelSide::Left => &mut self.left_panel,
            PanelSide::Right => &mut self.right_panel,
        }
    }

    fn get_inactive_panel(&mut self) -> &mut Panel {
        match self.active_panel {
            PanelSide::Left => &mut self.right_panel,
            PanelSide::Right => &mut self.left_panel,
        }
    }

    fn switch_panel(&mut self) {
        self.active_panel = match self.active_panel {
            PanelSide::Left => PanelSide::Right,
            PanelSide::Right => PanelSide::Left,
        };
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let app = App::new()?;
    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend + Write>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if handle_overlay_keys(&mut app, key.code) {
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::F(10) => return Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(())
                }
                KeyCode::Down => {
                    app.get_active_panel().next();
                }
                KeyCode::Up => {
                    app.get_active_panel().previous();
                }
                KeyCode::Enter => {
                    app.get_active_panel().enter()?;
                }
                KeyCode::Tab => {
                    app.switch_panel();
                }
                KeyCode::Left => {
                    app.active_panel = PanelSide::Left;
                }
                KeyCode::Right => {
                    app.active_panel = PanelSide::Right;
                }
                KeyCode::F(1) => {
                    app.show_help = !app.show_help;
                }
                KeyCode::F(3) => {
                    handle_view(&mut app)?;
                }
                KeyCode::F(4) => {
                    handle_edit(terminal, &mut app)?;
                }
                KeyCode::F(5) => {
                    handle_copy(&mut app)?;
                }
                KeyCode::F(6) => {
                    handle_move(&mut app)?;
                }
                KeyCode::F(7) => {
                    handle_mkdir(&mut app)?;
                }
                KeyCode::F(8) => {
                    handle_delete(&mut app)?;
                }
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
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

    // Render left panel
    render_panel(
        f,
        &mut app.left_panel,
        panels[0],
        app.active_panel == PanelSide::Left,
    );

    // Render right panel
    render_panel(
        f,
        &mut app.right_panel,
        panels[1],
        app.active_panel == PanelSide::Right,
    );

    // Render command line / status bar
    render_status(f, app, chunks[2]);
    render_command_bar(f, chunks[3]);
    render_overlays(f, app);
}

fn render_panel(f: &mut Frame, panel: &mut Panel, area: Rect, is_active: bool) {
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
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
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(size_str, Style::default().fg(Color::Gray)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut panel.state);
}

fn render_header(f: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new("Midnight Commander style | Tab: Switch panels | F1 for help")
        .style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
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
        .style(Style::default().bg(Color::Rgb(0, 0, 80)).fg(Color::White))
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(paragraph, area);
}

fn render_command_bar(f: &mut Frame, area: Rect) {
    let keys = [
        ("F1", "Help"),
        ("F3", "View"),
        ("F4", "Edit"),
        ("F5", "Copy"),
        ("F6", "Move"),
        ("F7", "Mkdir"),
        ("F8", "Delete"),
        ("F10", "Quit"),
    ];

    let spans: Vec<Span> = keys
        .iter()
        .map(|(key, label)| {
            Span::styled(
                format!(" {} {} ", key, label),
                Style::default()
                    .bg(Color::Rgb(0, 0, 80))
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Rgb(0, 0, 80)).fg(Color::White))
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(paragraph, area);
}

fn render_overlays(f: &mut Frame, app: &mut App) {
    if app.show_help {
        let area = centered_rect(80, 80, f.size());
        let help_lines = vec![
            "Midnight Commander style navigation:",
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
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Black));

        let paragraph = Paragraph::new(content)
            .block(block)
            .style(Style::default().fg(Color::White));

        f.render_widget(paragraph, area);
    }

    if let Some(viewer) = app.viewer.as_ref() {
        let area = centered_rect(90, 90, f.size());

        let block = Block::default()
            .title(format!(" View: {} (Esc to close) ", viewer.title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let paragraph = Paragraph::new(
            viewer
                .lines
                .iter()
                .map(|line| Line::from(Span::raw(line.clone())))
                .collect::<Vec<Line>>(),
        )
        .scroll((viewer.scroll, 0))
        .block(block)
        .style(Style::default().fg(Color::White));

        f.render_widget(paragraph, area);
    }
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

fn handle_overlay_keys(app: &mut App, key: KeyCode) -> bool {
    if let Some(viewer) = app.viewer.as_mut() {
        match key {
            KeyCode::Esc | KeyCode::F(3) | KeyCode::Enter => {
                app.viewer = None;
            }
            KeyCode::Down => {
                let max_scroll = viewer
                    .lines
                    .len()
                    .saturating_sub(1)
                    .saturating_sub(viewer.scroll as usize);
                if max_scroll > 0 {
                    viewer.scroll = viewer.scroll.saturating_add(1);
                }
            }
            KeyCode::Up => {
                if viewer.scroll > 0 {
                    viewer.scroll -= 1;
                }
            }
            _ => {}
        }
        return true;
    }

    if app.show_help {
        if matches!(key, KeyCode::Esc | KeyCode::F(1)) {
            app.show_help = false;
        }
        return true;
    }

    false
}

fn handle_view(app: &mut App) -> io::Result<()> {
    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    if selected.is_dir() {
        app.status_message = "Cannot view a directory".to_string();
        return Ok(());
    }

    let content = fs::read_to_string(&selected)?;
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    if lines.is_empty() {
        lines.push(String::from("<empty file>"));
    }

    app.viewer = Some(Viewer {
        title: selected
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        lines,
        scroll: 0,
    });
    app.status_message = format!("Viewing {}", selected.display());
    Ok(())
}

fn handle_edit<B: Backend + Write>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    if selected.is_dir() {
        app.status_message = "Editing directories is not supported".to_string();
        return Ok(());
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
    let status = Command::new(editor).arg(&selected).status();

    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
    terminal.clear()?;

    match status {
        Ok(code) => {
            if code.success() {
                app.status_message = format!("Edited {}", selected.display());
            } else {
                app.status_message = format!("Editor exited with status {}", code);
            }
        }
        Err(err) => {
            app.status_message = format!("Failed to launch editor: {}", err);
        }
    }

    app.left_panel.load_entries()?;
    app.left_panel.ensure_selection();
    app.right_panel.load_entries()?;
    app.right_panel.ensure_selection();
    Ok(())
}

fn handle_copy(app: &mut App) -> io::Result<()> {
    let Some(source_path) = app.get_active_panel().selected_path() else {
        return Ok(());
    };
    let target_dir = app.get_inactive_panel().path.clone();

    let dest = unique_destination(&target_dir, source_path.file_name().unwrap_or_default());
    if source_path.is_dir() {
        copy_dir_all(&source_path, &dest)?;
    } else {
        fs::copy(&source_path, &dest)?;
    }

    app.status_message = format!("Copied to {}", dest.display());
    refresh_panels(app)?;
    Ok(())
}

fn handle_move(app: &mut App) -> io::Result<()> {
    let Some(source_path) = app.get_active_panel().selected_path() else {
        return Ok(());
    };
    let target_dir = app.get_inactive_panel().path.clone();
    let dest = unique_destination(&target_dir, source_path.file_name().unwrap_or_default());

    match fs::rename(&source_path, &dest) {
        Ok(_) => {}
        Err(_) => {
            if source_path.is_dir() {
                copy_dir_all(&source_path, &dest)?;
                fs::remove_dir_all(&source_path)?;
            } else {
                fs::copy(&source_path, &dest)?;
                fs::remove_file(&source_path)?;
            }
        }
    }

    app.status_message = format!("Moved to {}", dest.display());
    refresh_panels(app)?;
    Ok(())
}

fn handle_mkdir(app: &mut App) -> io::Result<()> {
    let base_dir = app.get_active_panel().path.clone();
    let mut counter = 0;
    let mut candidate = base_dir.join("new_folder");
    while candidate.exists() {
        counter += 1;
        candidate = base_dir.join(format!("new_folder_{}", counter));
    }

    fs::create_dir_all(&candidate)?;
    app.status_message = format!("Created directory {}", candidate.display());
    refresh_panels(app)?;
    Ok(())
}

fn handle_delete(app: &mut App) -> io::Result<()> {
    let Some(target) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    if target.is_dir() {
        fs::remove_dir_all(&target)?;
    } else {
        fs::remove_file(&target)?;
    }

    app.status_message = format!("Deleted {}", target.display());
    refresh_panels(app)?;
    Ok(())
}

fn unique_destination(base: &Path, file_name: &std::ffi::OsStr) -> PathBuf {
    let mut dest = base.join(file_name);
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let extension = Path::new(file_name).extension().and_then(|e| e.to_str());

    let mut counter = 1;
    while dest.exists() {
        let new_name = match extension {
            Some(ext) if !ext.is_empty() => format!("{}_{}.{}", stem, counter, ext),
            _ => format!("{}_{}", stem, counter),
        };
        dest = base.join(new_name);
        counter += 1;
    }
    dest
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn refresh_panels(app: &mut App) -> io::Result<()> {
    app.left_panel.load_entries()?;
    app.left_panel.ensure_selection();
    app.right_panel.load_entries()?;
    app.right_panel.ensure_selection();
    Ok(())
}
