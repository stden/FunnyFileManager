mod handlers;
mod models;
mod ui;
mod utils;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{error::Error, io, io::Write};

pub use handlers::{
    handle_copy, handle_delete, handle_edit, handle_mkdir, handle_move, handle_overlay_keys,
    handle_view,
};
pub use models::{App, DirEntry, Panel, PanelSide, Viewer};
pub use ui::{centered_rect, ui, NAME_FIELD_WIDTH};
pub use utils::{copy_dir_all, format_size, refresh_panels, unique_destination};

fn main() -> Result<(), Box<dyn Error>> {
    main_impl()
}

fn main_impl() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new()?;
    let res = run_app(&mut terminal, app);

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

fn run_app<B: Backend + Write>(terminal: &mut Terminal<B>, app: App) -> io::Result<()> {
    run_app_with_event_reader(terminal, app, || event::read())
}

fn run_app_with_event_reader<B, F>(
    terminal: &mut Terminal<B>,
    mut app: App,
    mut read_event: F,
) -> io::Result<()>
where
    B: Backend + Write,
    F: FnMut() -> io::Result<Event>,
{
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = read_event()? {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::widgets::ListState;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1572864), "1.50 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
        assert_eq!(format_size(1610612736), "1.50 GB");
    }

    #[test]
    fn test_dir_entry() {
        let entry = DirEntry {
            name: "test.txt".to_string(),
            is_dir: false,
            size: 1024,
        };
        assert_eq!(entry.name, "test.txt");
        assert!(!entry.is_dir);
        assert_eq!(entry.size, 1024);
    }

    #[test]
    fn test_viewer_default() {
        let viewer = Viewer::default();
        assert_eq!(viewer.title, "");
        assert_eq!(viewer.lines.len(), 0);
        assert_eq!(viewer.scroll, 0);
    }

    #[test]
    fn test_panel_new() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let panel = Panel::new(temp_dir.path().to_path_buf())?;
        assert_eq!(panel.path, temp_dir.path());
        Ok(())
    }

    #[test]
    fn test_panel_load_entries() -> io::Result<()> {
        let temp_dir = TempDir::new()?;

        fs::create_dir(temp_dir.path().join("subdir1"))?;
        fs::create_dir(temp_dir.path().join("subdir2"))?;
        File::create(temp_dir.path().join("file1.txt"))?.write_all(b"test")?;
        File::create(temp_dir.path().join("file2.txt"))?.write_all(b"test content")?;

        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;
        panel.load_entries()?;

        assert!(panel.entries.len() >= 4);

        let mut found_file = false;
        for entry in &panel.entries {
            if !entry.is_dir && entry.name != ".." {
                found_file = true;
            }
            if found_file && entry.is_dir && entry.name != ".." {
                panic!("Found directory after file in sorted entries");
            }
        }

        Ok(())
    }

    #[test]
    fn test_panel_navigation() -> io::Result<()> {
        let temp_dir = TempDir::new()?;

        for i in 0..5 {
            File::create(temp_dir.path().join(format!("file{}.txt", i)))?;
        }

        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

        assert_eq!(panel.state.selected(), Some(0));

        panel.next();
        assert_eq!(panel.state.selected(), Some(1));

        panel.previous();
        assert_eq!(panel.state.selected(), Some(0));

        panel.previous();
        assert_eq!(panel.state.selected(), Some(panel.entries.len() - 1));

        panel.next();
        assert_eq!(panel.state.selected(), Some(0));

        Ok(())
    }

    #[test]
    fn test_panel_enter_directory() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir)?;

        let mut panel = Panel::new(temp_dir.path().to_path_buf())?;

        for (i, entry) in panel.entries.iter().enumerate() {
            if entry.name == "subdir" {
                panel.state.select(Some(i));
                break;
            }
        }

        panel.enter()?;
        assert_eq!(panel.path, subdir);

        assert!(panel.entries.iter().any(|e| e.name == ".."));

        panel.state.select(Some(0));
        panel.enter()?;
        assert_eq!(panel.path, temp_dir.path());

        Ok(())
    }

    #[test]
    fn test_app_new() -> io::Result<()> {
        let app = App::new()?;
        assert_eq!(app.active_panel, PanelSide::Left);
        assert!(!app.show_help);
        assert!(app.viewer.is_none());
        Ok(())
    }

    #[test]
    fn test_app_switch_panel() -> io::Result<()> {
        let mut app = App::new()?;
        assert_eq!(app.active_panel, PanelSide::Left);

        app.switch_panel();
        assert_eq!(app.active_panel, PanelSide::Right);

        app.switch_panel();
        assert_eq!(app.active_panel, PanelSide::Left);

        Ok(())
    }

    #[test]
    fn test_unique_destination() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let base = temp_dir.path();

        let dest1 = unique_destination(base, std::ffi::OsStr::new("test.txt"));
        assert_eq!(dest1, base.join("test.txt"));

        File::create(base.join("test.txt"))?;
        let dest2 = unique_destination(base, std::ffi::OsStr::new("test.txt"));
        assert_eq!(dest2, base.join("test_1.txt"));

        File::create(base.join("test_1.txt"))?;
        let dest3 = unique_destination(base, std::ffi::OsStr::new("test.txt"));
        assert_eq!(dest3, base.join("test_2.txt"));

        Ok(())
    }

    #[test]
    fn test_copy_dir_all() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let src = temp_dir.path().join("src");
        let dst = temp_dir.path().join("dst");

        fs::create_dir(&src)?;
        fs::create_dir(src.join("subdir"))?;
        File::create(src.join("file1.txt"))?.write_all(b"content1")?;
        File::create(src.join("subdir/file2.txt"))?.write_all(b"content2")?;

        copy_dir_all(&src, &dst)?;

        assert!(dst.exists());
        assert!(dst.join("file1.txt").exists());
        assert!(dst.join("subdir").exists());
        assert!(dst.join("subdir/file2.txt").exists());

        let content1 = fs::read_to_string(dst.join("file1.txt"))?;
        assert_eq!(content1, "content1");

        let content2 = fs::read_to_string(dst.join("subdir/file2.txt"))?;
        assert_eq!(content2, "content2");

        Ok(())
    }

    #[test]
    fn test_centered_rect() {
        let base = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };

        let centered = centered_rect(50, 50, base);
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 50);
        assert_eq!(centered.x, 25);
        assert_eq!(centered.y, 25);
    }

    #[test]
    fn test_handle_overlay_keys() -> io::Result<()> {
        let mut app = App::new()?;

        assert!(!handle_overlay_keys(&mut app, KeyCode::Esc));

        app.show_help = true;
        assert!(handle_overlay_keys(&mut app, KeyCode::Esc));
        assert!(!app.show_help);

        app.viewer = Some(Viewer {
            title: "test.txt".to_string(),
            lines: vec![
                "line1".to_string(),
                "line2".to_string(),
                "line3".to_string(),
            ],
            scroll: 0,
        });

        assert!(handle_overlay_keys(&mut app, KeyCode::Down));
        assert_eq!(app.viewer.as_ref().unwrap().scroll, 1);

        assert!(handle_overlay_keys(&mut app, KeyCode::Up));
        assert_eq!(app.viewer.as_ref().unwrap().scroll, 0);

        assert!(handle_overlay_keys(&mut app, KeyCode::Esc));
        assert!(app.viewer.is_none());

        Ok(())
    }

    #[test]
    fn test_handle_view() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?.write_all(b"Line 1\nLine 2\nLine 3")?;

        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_view(&mut app)?;

        assert!(app.viewer.is_some());
        let viewer = app.viewer.as_ref().unwrap();
        assert_eq!(viewer.title, "test.txt");
        assert_eq!(viewer.lines.len(), 3);
        assert_eq!(viewer.lines[0], "Line 1");

        Ok(())
    }

    #[test]
    fn test_handle_copy() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir)?;
        fs::create_dir(&right_dir)?;

        let file_path = left_dir.join("test.txt");
        File::create(&file_path)?.write_all(b"test content")?;

        let mut app = App::new()?;
        app.left_panel = Panel::new(left_dir.clone())?;
        app.right_panel = Panel::new(right_dir.clone())?;

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_copy(&mut app)?;

        assert!(right_dir.join("test.txt").exists());
        let content = fs::read_to_string(right_dir.join("test.txt"))?;
        assert_eq!(content, "test content");

        Ok(())
    }

    #[test]
    fn test_handle_move() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir)?;
        fs::create_dir(&right_dir)?;

        let file_path = left_dir.join("test.txt");
        File::create(&file_path)?.write_all(b"test content")?;

        let mut app = App::new()?;
        app.left_panel = Panel::new(left_dir.clone())?;
        app.right_panel = Panel::new(right_dir.clone())?;

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_move(&mut app)?;

        assert!(!file_path.exists());
        assert!(right_dir.join("test.txt").exists());
        let content = fs::read_to_string(right_dir.join("test.txt"))?;
        assert_eq!(content, "test content");

        Ok(())
    }

    #[test]
    fn test_handle_mkdir() -> io::Result<()> {
        let temp_dir = TempDir::new()?;

        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

        handle_mkdir(&mut app)?;

        assert!(temp_dir.path().join("new_folder").exists());

        handle_mkdir(&mut app)?;
        assert!(temp_dir.path().join("new_folder_1").exists());

        Ok(())
    }

    #[test]
    fn test_handle_delete_file() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("test.txt");
        File::create(&file_path)?.write_all(b"test")?;

        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "test.txt" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_delete(&mut app)?;

        assert!(!file_path.exists());

        Ok(())
    }

    #[test]
    fn test_handle_delete_directory() -> io::Result<()> {
        let temp_dir = TempDir::new()?;
        let dir_path = temp_dir.path().join("subdir");
        fs::create_dir(&dir_path)?;
        File::create(dir_path.join("file.txt"))?;

        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;

        for (i, entry) in app.left_panel.entries.iter().enumerate() {
            if entry.name == "subdir" {
                app.left_panel.state.select(Some(i));
                break;
            }
        }

        handle_delete(&mut app)?;

        assert!(!dir_path.exists());

        Ok(())
    }

    #[test]
    fn test_refresh_panels() -> io::Result<()> {
        let temp_dir = TempDir::new()?;

        let mut app = App::new()?;
        app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
        app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;

        File::create(temp_dir.path().join("new_file.txt"))?;

        refresh_panels(&mut app)?;

        let left_has_file = app
            .left_panel
            .entries
            .iter()
            .any(|e| e.name == "new_file.txt");
        let right_has_file = app
            .right_panel
            .entries
            .iter()
            .any(|e| e.name == "new_file.txt");

        assert!(left_has_file);
        assert!(right_has_file);

        Ok(())
    }

    #[test]
    fn test_panel_side_equality() {
        assert_eq!(PanelSide::Left, PanelSide::Left);
        assert_eq!(PanelSide::Right, PanelSide::Right);
        assert_ne!(PanelSide::Left, PanelSide::Right);
    }

    #[test]
    fn test_render_functions() -> io::Result<()> {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend)?;
        let mut app = App::new()?;

        terminal.draw(|f| {
            ui(f, &mut app);
        })?;

        app.show_help = true;
        terminal.draw(|f| {
            ui(f, &mut app);
        })?;

        app.show_help = false;
        app.viewer = Some(Viewer {
            title: "test.txt".to_string(),
            lines: vec!["Test content".to_string()],
            scroll: 0,
        });

        terminal.draw(|f| {
            ui(f, &mut app);
        })?;

        Ok(())
    }

    impl App {
        pub fn new_with_path(path: PathBuf) -> io::Result<Self> {
            Ok(App {
                left_panel: Panel::new(path.clone())?,
                right_panel: Panel::new(path)?,
                active_panel: PanelSide::Left,
                status_message: String::new(),
                show_help: false,
                viewer: None,
            })
        }
    }
}
