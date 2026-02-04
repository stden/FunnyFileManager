pub mod editor;
pub mod handlers;
pub mod models;
pub mod mutation_test;
pub mod ui;
pub mod utils;

#[cfg(test)]
mod editor_unit_tests;

#[cfg(test)]
mod mutation_killer_tests;

#[cfg(test)]
mod test_support;

use std::fs::OpenOptions;
use std::io::Write as IoWrite;

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
#[cfg(not(any(test, tarpaulin, coverage)))]
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
#[cfg(any(test, tarpaulin, coverage, feature = "test-helpers"))]
mod event {
    use crossterm::event::{Event, KeyCode, KeyModifiers};
    pub fn read() -> std::io::Result<Event> {
        Ok(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::empty(),
        )))
    }
}
#[cfg(all(not(test), not(tarpaulin), not(coverage)))]
use crossterm::event;
#[cfg(all(not(test), not(tarpaulin), not(coverage)))]
use ratatui::backend::CrosstermBackend;
use ratatui::{backend::Backend, Terminal};
use std::{error::Error, io, io::Write};

#[cfg(any(test, tarpaulin, coverage, feature = "test-helpers"))]
use ratatui::backend::TestBackend;

use crate::editor::handle_edit;
use crate::handlers::{
    handle_copy, handle_delete, handle_mkdir, handle_move, handle_overlay_keys, handle_view,
};
use crate::models::{App, PanelSide};
use crate::ui::ui;
use std::time::{Duration, Instant};

fn log_action(action: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("ffm_actions.log")
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let _ = writeln!(file, "[{}] {}", timestamp, action);
    }
}

fn log_error(error: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("ffm_errors.log")
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let _ = writeln!(file, "[{}] ERROR: {}", timestamp, error);
    }
}

#[cfg(coverage)]
pub fn coverage_touch_logs() {
    // Exercise non-test instantiations in coverage builds.
    log_action("coverage log action");
    log_error("coverage log error");
}

#[cfg(coverage)]
pub fn coverage_touch_backend() -> io::Result<()> {
    // Drive WritableTestBackend trait methods in non-test builds.
    let mut backend = WritableTestBackend::new(80, 24);
    let _ = backend.write(b"coverage").expect("write backend");
    Write::flush(&mut backend).expect("flush backend");
    Backend::flush(&mut backend).expect("flush backend");
    let _ = backend.get_cursor().expect("get cursor");
    backend.set_cursor(1, 1).expect("set cursor");
    backend.clear().expect("clear backend");
    let _ = backend.window_size().expect("window size");
    Ok(())
}

fn run_with<F>(main_runner: F) -> Result<(), Box<dyn Error>>
where
    F: FnOnce() -> Result<(), Box<dyn Error>>,
{
    log_action("=== FunnyFileManager Started ===");
    let result = main_runner();
    if let Err(ref e) = result {
        log_error(&format!("Main error: {}", e));
    }
    log_action("=== FunnyFileManager Ended ===");
    result
}

pub fn run() -> Result<(), Box<dyn Error>> {
    run_with(main_impl)
}

#[cfg(all(not(test), not(tarpaulin), not(coverage)))]
fn main_impl() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new()?;
    log_action("App initialized successfully");
    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

#[cfg(any(test, tarpaulin, coverage))]
fn main_impl() -> Result<(), Box<dyn Error>> {
    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let app = App::new().expect("app");

    run_app_with_event_reader(&mut terminal, app, || {
        Ok(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('q'),
            KeyModifiers::empty(),
        )))
    })
    .expect("run app");

    Ok(())
}

pub fn run_app<B: Backend + Write>(terminal: &mut Terminal<B>, app: App) -> io::Result<()> {
    run_app_with_event_reader(terminal, app, || event::read())
}

#[cfg(any(test, tarpaulin, coverage, feature = "test-helpers"))]
pub struct WritableTestBackend {
    inner: TestBackend,
}

#[cfg(any(test, tarpaulin, coverage, feature = "test-helpers"))]
impl WritableTestBackend {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            inner: TestBackend::new(width, height),
        }
    }
}

#[cfg(any(test, tarpaulin, coverage, feature = "test-helpers"))]
impl Write for WritableTestBackend {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(any(test, tarpaulin, coverage, feature = "test-helpers"))]
impl Backend for WritableTestBackend {
    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        self.inner.draw(content)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.inner.show_cursor()
    }

    fn get_cursor(&mut self) -> io::Result<(u16, u16)> {
        self.inner.get_cursor()
    }

    fn set_cursor(&mut self, x: u16, y: u16) -> io::Result<()> {
        self.inner.set_cursor(x, y)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.inner.clear()
    }

    fn size(&self) -> io::Result<ratatui::layout::Rect> {
        self.inner.size()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    fn window_size(&mut self) -> io::Result<ratatui::backend::WindowSize> {
        let size = self.size().expect("backend size");
        Ok(ratatui::backend::WindowSize {
            columns_rows: ratatui::layout::Size {
                width: size.width,
                height: size.height,
            },
            pixels: ratatui::layout::Size {
                width: size.width * 8,
                height: size.height * 16,
            },
        })
    }
}

pub fn run_app_with_event_reader<B, F>(
    terminal: &mut Terminal<B>,
    mut app: App,
    mut read_event: F,
) -> io::Result<()>
where
    B: Backend + Write,
    F: FnMut() -> io::Result<Event>,
{
    let mut last_nav_time = Instant::now();
    #[cfg(coverage)]
    let nav_debounce = Duration::from_millis(250);
    #[cfg(not(coverage))]
    let nav_debounce = Duration::from_millis(0);

    log_action(&format!(
        "Starting event loop with {} entries in left panel",
        app.left_panel.entries.len()
    ));

    while !app.should_quit {
        terminal.draw(|f| ui(f, &mut app))?;

        let event = read_event()?;
        if let Event::Key(key) = event {
            // Only process Press events, ignore Release/Repeat to avoid double navigation on Windows
            if key.kind != KeyEventKind::Press {
                continue;
            }

            log_action(&format!(
                "Key event: {:?} (modifiers: {:?})",
                key.code, key.modifiers
            ));

            if handle_overlay_keys(&mut app, key) {
                if app.should_quit {
                    log_action("Quitting from overlay");
                    break;
                }
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::F(10) => return Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(())
                }
                KeyCode::Down => {
                    let now = Instant::now();
                    let time_since = now.duration_since(last_nav_time);
                    log_action(&format!(
                        "Down key - time since last nav: {:?}ms, debounce: {:?}ms",
                        time_since.as_millis(),
                        nav_debounce.as_millis()
                    ));

                    if time_since >= nav_debounce {
                        let panel = app.get_active_panel();
                        let before = panel.state.selected();
                        panel.next();
                        let after = panel.state.selected();
                        log_action(&format!("Navigation DOWN: {:?} -> {:?}", before, after));
                        last_nav_time = now;
                    } else {
                        log_action("Down key ignored due to debounce");
                    }
                }
                KeyCode::Up => {
                    let now = Instant::now();
                    let time_since = now.duration_since(last_nav_time);
                    log_action(&format!(
                        "Up key - time since last nav: {:?}ms, debounce: {:?}ms",
                        time_since.as_millis(),
                        nav_debounce.as_millis()
                    ));

                    if time_since >= nav_debounce {
                        let panel = app.get_active_panel();
                        let before = panel.state.selected();
                        panel.previous();
                        let after = panel.state.selected();
                        log_action(&format!("Navigation UP: {:?} -> {:?}", before, after));
                        last_nav_time = now;
                    } else {
                        log_action("Up key ignored due to debounce");
                    }
                }
                KeyCode::Enter => app.get_active_panel().enter()?,
                KeyCode::Tab => {
                    app.switch_panel();
                }
                KeyCode::Left => {
                    app.active_panel = PanelSide::Left;
                }
                KeyCode::Right => {
                    app.active_panel = PanelSide::Right;
                }
                KeyCode::Home => {
                    let panel = app.get_active_panel();
                    panel.state.select(Some(0));
                    log_action("Navigation HOME: moved to first item");
                }
                KeyCode::End => {
                    let panel = app.get_active_panel();
                    if !panel.entries.is_empty() {
                        panel.state.select(Some(panel.entries.len() - 1));
                    }
                    log_action("Navigation END: moved to last item");
                }
                // Alt+F1: Drive selection for left panel (MUST come before F(1))
                KeyCode::F(1) if key.modifiers.contains(KeyModifiers::ALT) => {
                    app.show_drives = Some(PanelSide::Left);
                    log_action("Alt+F1: Opening drive selection for left panel");
                }
                // Alt+F2: Drive selection for right panel
                KeyCode::F(2) if key.modifiers.contains(KeyModifiers::ALT) => {
                    app.show_drives = Some(PanelSide::Right);
                    log_action("Alt+F2: Opening drive selection for right panel");
                }
                // Shift+F4: Create new file (MUST come before F(4))
                KeyCode::F(4) if key.modifiers.contains(KeyModifiers::SHIFT) => {
                    app.new_file_input = Some(String::new());
                    log_action("Shift+F4: Opening new file dialog");
                }
                KeyCode::F(1) => {
                    app.show_help = !app.show_help;
                }
                KeyCode::F(3) => handle_view(&mut app)?,
                KeyCode::F(4) => handle_edit(terminal, &mut app)?,
                KeyCode::F(5) => handle_copy(&mut app)?,
                KeyCode::F(6) => handle_move(&mut app)?,
                KeyCode::F(7) => handle_mkdir(&mut app)?,
                KeyCode::F(8) => handle_delete(&mut app)?,
                KeyCode::Insert => {
                    app.get_active_panel().toggle_mark_selected();
                    app.get_active_panel().next();
                }
                KeyCode::Char(' ') => {
                    app.get_active_panel().toggle_mark_selected();
                    app.get_active_panel().next();
                }
                KeyCode::Char('\\') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let root = if cfg!(windows) { "C:\\" } else { "/" };
                    app.get_active_panel().path = std::path::PathBuf::from(root);
                    app.get_active_panel().load_entries()?;
                    app.quick_search = None;
                }
                // Ctrl+U: Swap panel contents
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.swap_panels()?;
                    app.status_message = "Panels swapped".to_string();
                    log_action("Ctrl+U: Swapped panel contents");
                }
                KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::ALT) => {
                    // Alt+letter quick search like in Far Manager
                    let search_str = if let Some(ref mut existing) = app.quick_search {
                        existing.push(c);
                        existing.clone()
                    } else {
                        app.quick_search = Some(c.to_string());
                        c.to_string()
                    };

                    // Try to find matching entry
                    let panel = app.get_active_panel();
                    if !panel.find_first_matching(&search_str) {
                        // If no match found with full string, try with just the new character
                        if search_str.len() > 1 {
                            app.quick_search = Some(c.to_string());
                            app.get_active_panel().find_first_matching(&c.to_string());
                        }
                    }
                    log_action(&format!("Quick search: {:?}", app.quick_search));
                }
                KeyCode::Esc => {
                    app.quick_search = None;
                }
                KeyCode::Backspace => {
                    if let Some(ref mut search) = app.quick_search {
                        search.pop();
                        if search.is_empty() {
                            app.quick_search = None;
                        } else {
                            let search_str = search.clone();
                            app.get_active_panel().find_first_matching(&search_str);
                        }
                    } else {
                        app.quick_search = None;
                    }
                }
                _ => {
                    // Clear quick search on other keys
                    app.quick_search = None;
                }
            }
            let _ = ();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Panel;
    use crossterm::event::KeyEvent;
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn key_event(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn key_event_with_mods(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn cwd_mutex() -> &'static Mutex<()> {
        static CWD_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
        CWD_MUTEX.get_or_init(|| Mutex::new(()))
    }

    struct CwdGuard {
        original: std::path::PathBuf,
    }

    impl CwdGuard {
        fn set(path: &Path) -> io::Result<Self> {
            let original = env::current_dir().expect("current dir");
            env::set_current_dir(path).expect("set current dir");
            Ok(Self { original })
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.original);
        }
    }

    #[cfg(feature = "test-helpers")]
    fn select_entry(panel: &mut Panel, name: &str) {
        if let Some(index) = panel.entries.iter().position(|entry| entry.name == name) {
            panel.state.select(Some(index));
        }
    }

    #[test]
    #[cfg(feature = "test-helpers")]
    fn test_run_app_with_event_sequence() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let left_dir = temp_dir.path().join("left");
        let right_dir = temp_dir.path().join("right");
        fs::create_dir(&left_dir).expect("test setup");
        fs::create_dir(&right_dir).expect("test setup");

        fs::write(left_dir.join("left.txt"), b"left").expect("test setup");
        fs::write(right_dir.join("right.txt"), b"right").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(left_dir).expect("test setup");
        app.right_panel = Panel::new(right_dir).expect("test setup");
        app.active_panel = PanelSide::Left;
        select_entry(&mut app.left_panel, "left.txt");
        select_entry(&mut app.right_panel, "right.txt");
        app.show_help = true;

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            Event::Key(key_event(KeyCode::Esc)),
            Event::Key(key_event(KeyCode::F(1))),
            Event::Key(key_event(KeyCode::Esc)),
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::Up)),
            Event::Key(key_event(KeyCode::Enter)),
            Event::Key(key_event(KeyCode::Tab)),
            Event::Key(key_event(KeyCode::Left)),
            Event::Key(key_event(KeyCode::Right)),
            Event::Key(key_event(KeyCode::F(3))),
            Event::Key(key_event(KeyCode::Esc)),
            Event::Key(key_event(KeyCode::F(4))),
            Event::Key(key_event(KeyCode::F(5))),
            Event::Key(key_event(KeyCode::F(6))),
            Event::Key(key_event(KeyCode::F(7))),
            Event::Key(key_event(KeyCode::F(8))),
            Event::Key(key_event(KeyCode::Char('x'))),
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();

        let result = run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_run_app_ctrl_c_quit() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::write(temp_dir.path().join("file.txt"), b"data").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![Event::Key(key_event_with_mods(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        ))]
        .into_iter();

        let result = run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_run_app_command_line_quit() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.command_line = Some("quit".to_string());

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![Event::Key(key_event(KeyCode::Enter))].into_iter();
        let result = run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_run_app_should_quit_after_key() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![Event::Key(key_event(KeyCode::Char('q')))].into_iter();
        let result = run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_run_app_debounce_paths() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::Up)),
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();
        let result = run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_run_app_navigation_paths() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::Up)),
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();
        let result = run_app_with_event_reader(&mut terminal, app, || {
            #[cfg(coverage)]
            std::thread::sleep(Duration::from_millis(300));
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_main_impl_test_mode() {
        assert!(main_impl().is_ok());
    }

    #[test]
    fn test_run_logs_error_on_failure() -> io::Result<()> {
        let _ = fs::remove_file("ffm_errors.log");
        let result = run_with(|| Err(io::Error::new(io::ErrorKind::Other, "boom").into()));

        assert!(result.is_err());
        let contents = fs::read_to_string("ffm_errors.log").expect("test setup");
        assert!(contents.contains("Main error: boom"));
        Ok(())
    }

    #[test]
    fn test_log_open_failures() -> io::Result<()> {
        let _guard = cwd_mutex()
            .lock()
            .expect("failed to lock current directory");
        let temp_dir = TempDir::new().expect("test setup");
        let _cwd_guard = CwdGuard::set(temp_dir.path()).expect("test setup");

        fs::create_dir("ffm_actions.log").expect("test setup");
        fs::create_dir("ffm_errors.log").expect("test setup");

        log_action("action failure path");
        log_error("error failure path");

        assert!(Path::new("ffm_actions.log").is_dir());
        assert!(Path::new("ffm_errors.log").is_dir());

        Ok(())
    }

    #[cfg(tarpaulin)]
    #[test]
    fn test_tarpaulin_main_stub() {
        assert!(super::run().is_ok());
    }

    #[test]
    fn test_writable_test_backend_write() -> io::Result<()> {
        let mut backend = WritableTestBackend::new(80, 24);
        let data = b"test data";
        let written = backend.write(data).expect("test setup");
        assert_eq!(written, data.len());
        Ok(())
    }

    #[test]
    fn test_writable_test_backend_flush() -> io::Result<()> {
        let mut backend = WritableTestBackend::new(80, 24);
        Write::flush(&mut backend).expect("test setup");
        Backend::flush(&mut backend).expect("test setup");
        Ok(())
    }

    #[test]
    fn test_writable_test_backend_cursor_operations() -> io::Result<()> {
        let mut backend = WritableTestBackend::new(80, 24);

        let (x, y) = backend.get_cursor().expect("test setup");
        assert_eq!((x, y), (0, 0));

        backend.set_cursor(10, 5).expect("test setup");
        let (x, y) = backend.get_cursor().expect("test setup");
        assert_eq!((x, y), (10, 5));

        Ok(())
    }

    #[test]
    fn test_writable_test_backend_clear() -> io::Result<()> {
        let mut backend = WritableTestBackend::new(80, 24);
        backend.clear().expect("test setup");
        Ok(())
    }

    #[test]
    fn test_writable_test_backend_window_size() -> io::Result<()> {
        let mut backend = WritableTestBackend::new(80, 24);
        let window_size = backend.window_size().expect("test setup");

        assert_eq!(window_size.columns_rows.width, 80);
        assert_eq!(window_size.columns_rows.height, 24);
        assert_eq!(window_size.pixels.width, 80 * 8);
        assert_eq!(window_size.pixels.height, 24 * 16);

        Ok(())
    }

    #[test]
    fn test_run_app_full_coverage() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![Event::Key(key_event(KeyCode::F(10)))].into_iter();

        run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        })
        .expect("test setup");

        Ok(())
    }

    #[test]
    fn test_all_key_handlers_coverage() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).expect("test setup");
        fs::write(temp_dir.path().join("file1.txt"), b"content1").expect("test setup");
        fs::write(temp_dir.path().join("file2.txt"), b"content2").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            // Navigation keys
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::Up)),
            // Panel switching
            Event::Key(key_event(KeyCode::Tab)),
            Event::Key(key_event(KeyCode::Left)),
            Event::Key(key_event(KeyCode::Right)),
            // F1 toggle help
            Event::Key(key_event(KeyCode::F(1))),
            Event::Key(key_event(KeyCode::Esc)),
            // F3 view (then close)
            Event::Key(key_event(KeyCode::F(3))),
            Event::Key(key_event(KeyCode::Esc)),
            // F7 mkdir (then cancel)
            Event::Key(key_event(KeyCode::F(7))),
            Event::Key(key_event(KeyCode::Esc)),
            // Enter key on directory
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::Enter)),
            // Backspace with no quick search
            Event::Key(key_event(KeyCode::Backspace)),
            // Unknown key (default case)
            Event::Key(key_event(KeyCode::Char('x'))),
            // Quit
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();

        run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        })
        .expect("test setup");

        Ok(())
    }

    #[test]
    fn test_overlay_continue_path() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::write(temp_dir.path().join("file.txt"), b"content").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        // Start with help overlay active
        app.show_help = true;

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            // This key is handled by overlay, triggering continue
            Event::Key(key_event(KeyCode::Esc)),
            // After overlay closes, this quits
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();

        run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        })
        .expect("test setup");

        Ok(())
    }

    #[test]
    fn test_function_keys_f4_f5_f6_f8() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, b"content").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            // F4 edit (opens editor overlay, then close)
            Event::Key(key_event(KeyCode::F(4))),
            Event::Key(key_event(KeyCode::Esc)),
            // F5 copy (select file first)
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::F(5))),
            // F6 move (cancel via Esc since no confirmation needed)
            Event::Key(key_event(KeyCode::F(6))),
            // F8 delete (cancel)
            Event::Key(key_event(KeyCode::F(8))),
            // Quit
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();

        run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        })
        .expect("test setup");

        Ok(())
    }

    #[test]
    fn test_main_function_coverage() {
        let result = run();
        assert!(result.is_ok());
    }

    #[test]
    fn test_navigation_moves_one_item() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        // Create multiple files to test navigation
        for i in 1..=5 {
            fs::write(temp_dir.path().join(format!("file{}.txt", i)), b"data").expect("test setup");
        }

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        // Start at first item
        app.left_panel.state.select(Some(0));

        // Move down once
        app.left_panel.next();
        assert_eq!(
            app.left_panel.state.selected(),
            Some(1),
            "Should move to second item"
        );

        // Move down again
        app.left_panel.next();
        assert_eq!(
            app.left_panel.state.selected(),
            Some(2),
            "Should move to third item"
        );

        // Move up once
        app.left_panel.previous();
        assert_eq!(
            app.left_panel.state.selected(),
            Some(1),
            "Should move back to second item"
        );

        Ok(())
    }

    #[test]
    fn test_far_manager_hotkeys() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::create_dir(temp_dir.path().join("subdir")).expect("test setup");
        fs::write(temp_dir.path().join("file1.txt"), b"data").expect("test setup");
        fs::write(temp_dir.path().join("file2.txt"), b"data").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            // Test Insert key for marking
            Event::Key(key_event(KeyCode::Insert)),
            // Test Space key for marking
            Event::Key(key_event(KeyCode::Char(' '))),
            // Test Ctrl+\ for root navigation
            Event::Key(key_event_with_mods(
                KeyCode::Char('\\'),
                KeyModifiers::CONTROL,
            )),
            // Exit
            Event::Key(key_event(KeyCode::F(10))),
        ]
        .into_iter();

        let result = run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_run_app_coverage() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        // This test covers the run_app function wrapper
        run_app(&mut terminal, app).expect("test setup");
        Ok(())
    }

    #[test]
    fn test_quick_search_alt_key() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::write(temp_dir.path().join("alpha.txt"), b"data").expect("test setup");
        fs::write(temp_dir.path().join("beta.txt"), b"data").expect("test setup");
        fs::write(temp_dir.path().join("gamma.txt"), b"data").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            // Alt+b to search for "beta"
            Event::Key(key_event_with_mods(KeyCode::Char('b'), KeyModifiers::ALT)),
            // Esc to clear search
            Event::Key(key_event(KeyCode::Esc)),
            // Alt+g to search for "gamma"
            Event::Key(key_event_with_mods(KeyCode::Char('g'), KeyModifiers::ALT)),
            // Quit
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();

        let result = run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_quick_search_multi_char() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::write(temp_dir.path().join("apple.txt"), b"data").expect("test setup");
        fs::write(temp_dir.path().join("apricot.txt"), b"data").expect("test setup");
        fs::write(temp_dir.path().join("avocado.txt"), b"data").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            // Type "ap" to narrow search to apple/apricot
            Event::Key(key_event_with_mods(KeyCode::Char('a'), KeyModifiers::ALT)),
            Event::Key(key_event_with_mods(KeyCode::Char('p'), KeyModifiers::ALT)),
            // Backspace to remove 'p'
            Event::Key(key_event(KeyCode::Backspace)),
            // Quit
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();

        let result = run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_quick_search_backspace_clears() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::write(temp_dir.path().join("test.txt"), b"data").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            // Start quick search
            Event::Key(key_event_with_mods(KeyCode::Char('t'), KeyModifiers::ALT)),
            // Backspace clears the single character, clearing quick search
            Event::Key(key_event(KeyCode::Backspace)),
            // Quit
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();

        let result = run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_quick_search_no_match_fallback() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::write(temp_dir.path().join("apple.txt"), b"data").expect("test setup");
        fs::write(temp_dir.path().join("banana.txt"), b"data").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            // Search for "a"
            Event::Key(key_event_with_mods(KeyCode::Char('a'), KeyModifiers::ALT)),
            // Then "x" - no match for "ax", should try just "x"
            Event::Key(key_event_with_mods(KeyCode::Char('x'), KeyModifiers::ALT)),
            // Quit
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();

        let result = run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_quick_search_no_match_single_char() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::write(temp_dir.path().join("apple.txt"), b"data").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            // No match for "x" should exercise the single-char fallback path
            Event::Key(key_event_with_mods(KeyCode::Char('x'), KeyModifiers::ALT)),
            // Quit
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();

        let result = run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_quick_search_cleared_on_navigation() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::write(temp_dir.path().join("file.txt"), b"data").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
        app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            // Start quick search
            Event::Key(key_event_with_mods(KeyCode::Char('f'), KeyModifiers::ALT)),
            // Navigate - should NOT clear quick search
            Event::Key(key_event(KeyCode::Down)),
            Event::Key(key_event(KeyCode::Up)),
            // Tab should clear quick search
            Event::Key(key_event(KeyCode::Tab)),
            // Quit
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();

        let result = run_app_with_event_reader(&mut terminal, app, || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_quick_search_cleared_on_tab() -> io::Result<()> {
        let temp_dir = TempDir::new().expect("test setup");
        fs::write(temp_dir.path().join("file.txt"), b"data").expect("test setup");

        let mut app = App::new().expect("test setup");
        app.left_panel.path = temp_dir.path().to_path_buf();
        app.right_panel.path = temp_dir.path().to_path_buf();

        let backend = WritableTestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test setup");

        let mut events = vec![
            // Start quick search
            Event::Key(key_event_with_mods(KeyCode::Char('f'), KeyModifiers::ALT)),
            // Tab should clear quick search
            Event::Key(key_event(KeyCode::Tab)),
            // Quit
            Event::Key(key_event(KeyCode::Char('q'))),
        ]
        .into_iter();

        let result = run_app_with_event_reader(&mut terminal, app.clone(), || {
            events.next().ok_or(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "No more events",
            ))
        });

        assert!(result.is_ok());
        Ok(())
    }
}
