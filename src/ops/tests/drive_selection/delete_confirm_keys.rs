//! Purpose: Keys in the delete confirmation: y, Y, Enter, n, N, Esc and the rest.

use super::*;

// -----------------------------------------------------------------------
// Delete confirmation overlay – additional branch coverage
// -----------------------------------------------------------------------

// --- Delete confirm overlay keys ---

#[test]
fn test_process_popup_input_delete_confirm_enter() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del.txt");
    fs::write(&file_path, b"data").expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    // Enter key should confirm delete
    assert!(process_popup_input(&mut app, key_event(KeyCode::Enter)));
    assert!(!file_path.exists());
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));

    Ok(())
}

/// 'Y' (uppercase) confirms deletion.
#[test]
fn test_delete_confirm_uppercase_y() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del_Y.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(&mut app, key_event(KeyCode::Char('Y')));

    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(!file_path.exists());

    Ok(())
}

/// Enter key confirms deletion.
#[test]
fn test_delete_confirm_enter() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del_enter.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(!file_path.exists());

    Ok(())
}

#[test]
fn test_process_popup_input_delete_confirm_uppercase_n() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![PathBuf::from("/tmp/x")]));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Char('N'))));
    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(app.status_text().contains("cancelled"));

    Ok(())
}

/// 'N' (uppercase) cancels deletion.
#[test]
fn test_delete_confirm_uppercase_n() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del_N.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(&mut app, key_event(KeyCode::Char('N')));

    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(file_path.exists());
    assert!(app.status_text().contains("cancelled"));

    Ok(())
}

#[test]
fn test_process_popup_input_delete_confirm_other_key() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![PathBuf::from("/tmp/x")]));

    // Other key should be consumed but not act
    assert!(process_popup_input(&mut app, key_event(KeyCode::Down)));
    assert!(matches!(app.overlay, Overlay::DeleteConfirm(_)));

    Ok(())
}

/// Esc key cancels deletion.
#[test]
fn test_delete_confirm_esc_cancels() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del_esc.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    process_popup_input(&mut app, key_event(KeyCode::Esc));

    assert!(!matches!(app.overlay, Overlay::DeleteConfirm(_)));
    assert!(file_path.exists());
    assert!(app.status_text().contains("cancelled"));

    Ok(())
}

/// Any other key leaves `delete_confirm` unchanged (overlay stays open).
#[test]
fn test_delete_confirm_other_key_stays_open() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let file_path = temp_dir.path().join("del_other.txt");
    File::create(&file_path).expect("test setup");

    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::DeleteConfirm(vec![file_path.clone()]));

    // A key that is not y/Y/n/N/Enter/Esc.
    process_popup_input(&mut app, key_event(KeyCode::Char('x')));

    // delete_confirm must still be Some – the overlay stays open.
    assert!(matches!(app.overlay, Overlay::DeleteConfirm(_)));
    // File must not be deleted.
    assert!(file_path.exists());

    Ok(())
}
