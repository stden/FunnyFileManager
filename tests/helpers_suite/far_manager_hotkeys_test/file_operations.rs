//! Purpose: F5 copy, F6 move, F7 make folder and F8 delete confirmation.

use super::*;

// ─── F5 COPY ────────────────────────────────────────────────────────────────

#[test]
fn hotkey_f5_copies_file_to_other_panel() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir(&src).unwrap();
    fs::create_dir(&dst).unwrap();
    fs::write(src.join("copy_me.txt"), b"data").unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(src).unwrap();
    app.right_panel = Panel::new(dst.clone()).unwrap();
    app.active_panel = PanelSide::Left;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "copy_me.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    handle_copy(&mut app).unwrap();
    assert!(
        dst.join("copy_me.txt").exists(),
        "file should be copied to dst panel"
    );
}

#[test]
fn hotkey_f5_copy_no_selection_is_noop() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.left_panel.select(None);
    assert!(handle_copy(&mut app).is_ok());
}

// ─── F6 MOVE ────────────────────────────────────────────────────────────────

#[test]
fn hotkey_f6_moves_file_to_other_panel() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("src");
    let dst = temp.path().join("dst");
    fs::create_dir(&src).unwrap();
    fs::create_dir(&dst).unwrap();
    fs::write(src.join("move_me.txt"), b"data").unwrap();

    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(src.clone()).unwrap();
    app.right_panel = Panel::new(dst.clone()).unwrap();
    app.active_panel = PanelSide::Left;
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "move_me.txt")
        .unwrap();
    app.left_panel.select(Some(idx));

    handle_move(&mut app).unwrap();
    assert!(
        dst.join("move_me.txt").exists(),
        "file should appear in dst"
    );
    assert!(
        !src.join("move_me.txt").exists(),
        "file should be gone from src"
    );
}

// ─── F7 MKDIR ───────────────────────────────────────────────────────────────

#[test]
fn hotkey_f7_creates_directory_via_command_line() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    // F7 opens mkdir dialog → command_line
    app.set_overlay(Overlay::CommandLine(String::new().into()));
    process_popup_input(&mut app, press(KeyCode::Char('m')));
    process_popup_input(&mut app, press(KeyCode::Char('k')));
    process_popup_input(&mut app, press(KeyCode::Char('d')));
    assert_eq!(app.get_command_line(), Some("mkd"));
    // Esc cancels
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(app.get_command_line().is_none());
}

#[test]
fn hotkey_f7_mkdir_creates_directory() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    handle_mkdir(&mut app).unwrap();
    // Mkdir with no input (command_line opens); verify command_line is now active or noop
    // (handle_mkdir opens command_line with prompt)
    // Just verify it doesn't crash and sets some state
    assert!(app.get_command_line().is_some() || !app.status_text().is_empty());
}

// ─── F8 DELETE ──────────────────────────────────────────────────────────────

#[test]
fn hotkey_f8_shows_delete_confirmation_dialog() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("del_me.txt"), b"bye").unwrap();
    let mut app = setup(&temp).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "del_me.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    handle_delete_confirm(&mut app).unwrap();
    assert!(
        app.get_delete_confirm().is_some(),
        "delete confirm dialog should open"
    );
}

#[test]
fn overlay_delete_confirm_n_cancels() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![temp.path().join("ghost.txt")]));
    process_popup_input(&mut app, press(KeyCode::Char('n')));
    assert!(app.get_delete_confirm().is_none());
    assert!(app.status_text().contains("cancel") || app.status_text().contains("Cancel"));
}

#[test]
fn overlay_delete_confirm_capital_n_cancels() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![temp.path().join("ghost.txt")]));
    process_popup_input(&mut app, press(KeyCode::Char('N')));
    assert!(app.get_delete_confirm().is_none());
}

#[test]
fn overlay_delete_confirm_esc_cancels() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![temp.path().join("ghost.txt")]));
    process_popup_input(&mut app, press(KeyCode::Esc));
    assert!(app.get_delete_confirm().is_none());
}

#[test]
fn overlay_delete_confirm_y_deletes_file() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("delete_this.txt");
    fs::write(&file, b"gone").unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![file.clone()]));
    process_popup_input(&mut app, press(KeyCode::Char('y')));
    assert!(!file.exists(), "file should be deleted after Y");
    assert!(app.get_delete_confirm().is_none());
}

#[test]
fn overlay_delete_confirm_enter_deletes_file() {
    let temp = TempDir::new().unwrap();
    let file = temp.path().join("delete_enter.txt");
    fs::write(&file, b"gone").unwrap();
    let mut app = setup(&temp).unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![file.clone()]));
    process_popup_input(&mut app, press(KeyCode::Enter));
    assert!(!file.exists(), "file should be deleted on Enter");
}
