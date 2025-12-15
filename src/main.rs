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
    error::Error,
    fs,
    io,
    path::PathBuf,
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
        entries.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
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
}

struct App {
    left_panel: Panel,
    right_panel: Panel,
    active_panel: PanelSide,
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
        })
    }

    fn get_active_panel(&mut self) -> &mut Panel {
        match self.active_panel {
            PanelSide::Left => &mut self.left_panel,
            PanelSide::Right => &mut self.right_panel,
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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(()),
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
                _ => {}
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    // Render left panel
    render_panel(f, &mut app.left_panel, panels[0], app.active_panel == PanelSide::Left);
    
    // Render right panel
    render_panel(f, &mut app.right_panel, panels[1], app.active_panel == PanelSide::Right);

    // Render command line / status bar
    render_status(f, app, chunks[1]);
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
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
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
        " ↑↓: Navigate | Enter: Open | Tab: Switch Panel | ←→: Select Panel | Q: Quit | {}",
        selected_info
    );

    let paragraph = Paragraph::new(help_text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(paragraph, area);
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
