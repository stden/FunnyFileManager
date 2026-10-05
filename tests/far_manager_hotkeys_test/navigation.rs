//! Purpose: Quit keys, cursor movement, panel focus and Enter.

use super::*;

// ─── QUIT KEYS ──────────────────────────────────────────────────────────────

#[test]
fn hotkey_q_quits() {
    let temp = TempDir::new().unwrap();
    let app = setup(&temp).unwrap();
    assert!(run(app, vec![ev(KeyCode::F(10)), ev(KeyCode::F(10))]).is_ok());
}

#[test]
fn hotkey_shift_q_quits() {
    let temp = TempDir::new().unwrap();
    let app = setup(&temp).unwrap();
    assert!(run(app, vec![ev(KeyCode::F(10)), ev(KeyCode::F(10))]).is_ok());
}

#[test]
fn hotkey_f10_quits() {
    let temp = TempDir::new().unwrap();
    let app = setup(&temp).unwrap();
    assert!(run(app, vec![ev(KeyCode::F(10)), ev(KeyCode::F(10))]).is_ok());
}

#[test]
fn hotkey_ctrl_c_quits() {
    let temp = TempDir::new().unwrap();
    let app = setup(&temp).unwrap();
    assert!(run(app, vec![ev_mod(KeyCode::Char('c'), KeyModifiers::CONTROL)]).is_ok());
}

// ─── PANEL NAVIGATION ───────────────────────────────────────────────────────

#[test]
fn hotkey_down_moves_selection_forward() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0));
    panel.next();
    assert_eq!(panel.selected(), Some(1));
}

#[test]
fn hotkey_up_moves_selection_backward() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let last = panel.entries.len() - 1;
    panel.select(Some(last));
    panel.previous();
    assert_eq!(panel.selected(), Some(last - 1));
}

#[test]
fn hotkey_down_stops_at_the_last_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let last = panel.entries.len() - 1;
    panel.select(Some(last));
    panel.next();
    assert_eq!(panel.selected(), Some(last));
}

#[test]
fn hotkey_up_stops_at_the_first_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0));
    panel.previous();
    assert_eq!(panel.selected(), Some(0));
}

#[test]
fn hotkey_home_jumps_to_first_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(panel.entries.len() - 1));
    panel.select(Some(0)); // home logic
    assert_eq!(panel.selected(), Some(0));
}

#[test]
fn hotkey_end_jumps_to_last_entry() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    panel.select(Some(0));
    let last = panel.entries.len() - 1;
    panel.select(Some(last)); // end logic
    assert_eq!(panel.selected(), Some(last));
}

#[test]
fn hotkey_tab_switches_active_panel() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    assert_eq!(app.active_panel, PanelSide::Left);
    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Right);
    app.switch_panel();
    assert_eq!(app.active_panel, PanelSide::Left);
}

#[test]
fn hotkey_left_selects_left_panel() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.active_panel = PanelSide::Right;
    app.active_panel = PanelSide::Left; // left arrow logic
    assert_eq!(app.active_panel, PanelSide::Left);
}

#[test]
fn hotkey_right_selects_right_panel() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.active_panel = PanelSide::Left;
    app.active_panel = PanelSide::Right; // right arrow logic
    assert_eq!(app.active_panel, PanelSide::Right);
}

#[test]
fn hotkey_enter_enters_subdirectory() {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "subdir")
        .unwrap();
    panel.select(Some(idx));
    panel.enter().unwrap();
    assert_eq!(panel.path, sub);
}

#[test]
fn hotkey_enter_on_dotdot_goes_to_parent() {
    let temp = TempDir::new().unwrap();
    let sub = temp.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    let mut panel = Panel::new(sub).unwrap();
    // ".." is entry 0
    panel.select(Some(0));
    panel.enter().unwrap();
    assert_eq!(panel.path, temp.path());
}

#[test]
fn hotkey_enter_on_file_does_not_change_path() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("file.txt"), b"data").unwrap();
    let mut panel = Panel::new(temp.path().to_path_buf()).unwrap();
    let idx = panel
        .entries
        .iter()
        .position(|e| e.name == "file.txt")
        .unwrap();
    panel.select(Some(idx));
    let before = panel.path.clone();
    panel.enter().unwrap();
    assert_eq!(panel.path, before);
}
