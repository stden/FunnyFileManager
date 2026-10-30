//! Purpose: Ctrl+R re-reads both panels and reports success or failure.

use super::*;

// --- Ctrl+R refresh panels ---

#[test]
fn test_ctrl_r_refresh() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        )),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    let result = run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    });

    assert!(result.is_ok());
    Ok(())
}

#[test]
fn test_ctrl_r_refresh_renders_success_status() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        )),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    let screen = buffer_text(&terminal);
    assert!(screen.contains("Panels refreshed"), "{screen}");
    Ok(())
}

/// A reload that fails with nowhere to go says so. (A deleted absolute folder
/// is not a failure: the panel moves up, see the next test.) A relative path
/// that does not exist has no folder above it, so it still fails — without
/// the process-wide `FORCE_REFRESH_ERROR`, which would fail other tests too.
#[test]
fn test_ctrl_r_refresh_renders_failure_status() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.left_panel.path = PathBuf::from("no-such-folder-for-a-refresh-test");

    let backend = WritableTestBackend::new(240, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    let mut events = vec![
        Event::Key(key_event_with_mods(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL,
        )),
        Event::Key(key_event(KeyCode::F(10))),
        // F10 now asks before quitting; Enter answers Yes.
        Event::Key(key_event(KeyCode::Enter)),
    ]
    .into_iter();

    run_app_with_event_reader(&mut terminal, app, || {
        events
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "No more events"))
    })?;

    let screen = buffer_text(&terminal);
    assert!(screen.contains("Refresh failed"), "{screen}");
    Ok(())
}

/// Ctrl+R in a folder deleted under the panel moves it to the nearest folder
/// above that exists, instead of "Refresh failed" over the deleted listing.
#[test]
fn test_ctrl_r_leaves_a_deleted_folder_for_the_one_above() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let gone = temp_dir.path().join("gone");
    fs::create_dir_all(gone.join("inner")).expect("test setup");
    fs::write(gone.join("inner").join("g.txt"), b"g").expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(gone.join("inner")).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    fs::remove_dir_all(&gone).expect("test setup");

    let mut terminal = Terminal::new(WritableTestBackend::new(240, 24)).expect("test setup");
    // Held so a test forcing refresh failures cannot run meanwhile.
    let _refresh = crate::utils::refresh_panels_mutex()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::Char('r'), KeyModifiers::CONTROL),
    );

    assert_eq!(app.left_panel.path, temp_dir.path());
    assert!(!app
        .left_panel
        .entries
        .iter()
        .any(|entry| entry.name == "g.txt"));
    assert_eq!(app.status_text(), "Panels refreshed");
    Ok(())
}
