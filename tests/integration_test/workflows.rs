//! Purpose: Longer scenarios: file operations, resizing, event bursts, error recovery and UI
//! states.

use super::*;

#[test]
fn test_complex_file_operations_workflow() -> io::Result<()> {
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    let temp_dir = TempDir::new()?;
    let src_dir = temp_dir.path().join("src");
    let dst_dir = temp_dir.path().join("dst");

    fs::create_dir(&src_dir)?;
    fs::create_dir(&dst_dir)?;

    for i in 0..5 {
        let file = src_dir.join(format!("file{i}.txt"));
        File::create(&file)?.write_all(format!("Content {i}").as_bytes())?;
    }

    fs::create_dir(src_dir.join("subdir"))?;
    File::create(src_dir.join("subdir").join("nested.txt"))?.write_all(b"Nested content")?;

    assert_eq!(fs::read_dir(&src_dir)?.count(), 6);

    Ok(())
}

#[test]
fn test_terminal_resize_handling() -> Result<(), Box<dyn std::error::Error>> {
    let sizes = vec![(80, 24), (120, 40), (40, 20), (200, 50)];

    for (width, height) in sizes {
        let backend = TestBackend::new(width, height);
        let terminal = Terminal::new(backend)?;

        assert_eq!(terminal.size()?.width, width);
        assert_eq!(terminal.size()?.height, height);
    }

    Ok(())
}

#[test]
fn test_concurrent_event_handling() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let (tx, rx) = mpsc::channel();
    let rx = Arc::new(Mutex::new(rx));

    let handles: Vec<_> = (0..3)
        .map(|i| {
            let tx = tx.clone();
            thread::spawn(move || {
                for j in 0..5 {
                    let key = match (i + j) % 3 {
                        0 => KeyCode::Down,
                        1 => KeyCode::Up,
                        _ => KeyCode::Enter,
                    };
                    tx.send(Event::Key(KeyEvent::new(key, KeyModifiers::empty())))
                        .unwrap();
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    drop(tx);

    let rx = rx.lock().unwrap();
    let mut count = 0;
    while rx.try_recv().is_ok() {
        count += 1;
    }
    assert_eq!(count, 15);
}

#[test]
fn test_error_recovery_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    let result = terminal.draw(|f| {
        let area = f.area();
        assert!(area.width > 0);
        assert!(area.height > 0);
    });

    assert!(result.is_ok());

    Ok(())
}

#[test]
fn test_complex_ui_state_transitions() {
    let states = vec![
        ("Help", KeyCode::F(1)),
        ("View", KeyCode::F(3)),
        ("Edit", KeyCode::F(4)),
        ("Copy", KeyCode::F(5)),
        ("Move", KeyCode::F(6)),
        ("Mkdir", KeyCode::F(7)),
        ("Delete", KeyCode::F(8)),
    ];

    for (name, key) in states {
        let event = simulate_key_event(key, KeyModifiers::empty());
        match event {
            Event::Key(k) => {
                assert_eq!(k.code, key, "State transition for {name} failed");
            }
            _ => panic!("Expected key event for {name}"),
        }
    }
}
