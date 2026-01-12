use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::models::{App, Panel};
use funny_file_manager::run_app_with_event_reader;
use ratatui::{backend::TestBackend, Terminal};
use std::fs;
use std::io;
use tempfile::TempDir;

fn main() -> io::Result<()> {
    // Create test environment
    let temp_dir = TempDir::new()?;
    for i in 1..=10 {
        fs::write(temp_dir.path().join(format!("file{:02}.txt", i)), b"test")?;
    }

    let mut app = App::new()?;
    app.left_panel = Panel::new(temp_dir.path().to_path_buf())?;
    app.right_panel = Panel::new(temp_dir.path().to_path_buf())?;
    
    // Start at first file
    app.left_panel.state.select(Some(0));
    
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    // Create event sequence
    let mut events = vec![
        Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty())),
        Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty())),
        Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty())),
        Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty())),
        Event::Key(KeyEvent::new(KeyCode::F(10), KeyModifiers::empty())),
    ].into_iter();

    let mut event_count = 0;
    let mut positions = Vec::new();
    
    run_app_with_event_reader(&mut terminal, app, || {
        positions.push(terminal.get_frame().buffer.as_ref().unwrap().content.len());
        event_count += 1;
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    println!("Events processed: {}", event_count);
    println!("Positions recorded: {:?}", positions);
    
    Ok(())
}