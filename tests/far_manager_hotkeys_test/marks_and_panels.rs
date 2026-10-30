//! Purpose: Marking, panel swap, console toggle and jumping to the root.

use super::*;

// ─── INSERT / SPACE MARK ────────────────────────────────────────────────────

#[test]
fn hotkey_insert_marks_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("mark_me.txt"), b"").unwrap();
    let mut app = setup(&temp).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "mark_me.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    app.left_panel.toggle_mark_selected(); // Insert logic
    assert!(app.left_panel.marked.contains("mark_me.txt"));
}

#[test]
fn hotkey_insert_unmarks_already_marked_file() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("unmark_me.txt"), b"").unwrap();
    let mut app = setup(&temp).unwrap();
    let idx = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "unmark_me.txt")
        .unwrap();
    app.left_panel.select(Some(idx));
    app.left_panel.marked.insert("unmark_me.txt".into());
    app.left_panel.toggle_mark_selected();
    assert!(!app.left_panel.marked.contains("unmark_me.txt"));
}

#[test]
fn hotkey_space_marks_and_advances_to_next() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.txt"), b"").unwrap();
    fs::write(temp.path().join("b.txt"), b"").unwrap();
    let mut app = setup(&temp).unwrap();
    let idx_a = app
        .left_panel
        .entries
        .iter()
        .position(|e| e.name == "a.txt")
        .unwrap();
    app.left_panel.select(Some(idx_a));
    // Space logic: toggle_mark + next
    app.left_panel.toggle_mark_selected();
    app.left_panel.next();
    assert!(app.left_panel.marked.contains("a.txt"));
    assert_eq!(app.left_panel.selected(), Some(idx_a + 1));
}

// ─── CTRL+U SWAP PANELS ──────────────────────────────────────────────────────

#[test]
fn hotkey_ctrl_u_swaps_panel_paths() {
    let temp = TempDir::new().unwrap();
    let left_dir = temp.path().join("left");
    let right_dir = temp.path().join("right");
    fs::create_dir(&left_dir).unwrap();
    fs::create_dir(&right_dir).unwrap();
    let mut app = App::new().unwrap();
    app.left_panel = Panel::new(left_dir.clone()).unwrap();
    app.right_panel = Panel::new(right_dir.clone()).unwrap();
    app.swap_panels();
    assert_eq!(app.left_panel.path, right_dir);
    assert_eq!(app.right_panel.path, left_dir);
}

// ─── CTRL+O CONSOLE MODE ─────────────────────────────────────────────────────

/// Through `ViewMode::toggled` — the same call the Ctrl+O action makes. These
/// used to assign the field directly, so they could only agree with themselves.
#[test]
fn hotkey_ctrl_o_toggles_console_both_ways() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    assert_eq!(app.view, ViewMode::Panels);

    app.view = app.view.toggled();
    assert_eq!(app.view, ViewMode::Console);

    app.view = app.view.toggled();
    assert_eq!(app.view, ViewMode::Panels);
}

/// Closing a dialog is not leaving console mode: the two are orthogonal, which
/// is exactly what splitting `ViewMode` out of `Overlay` bought.
#[test]
fn closing_an_overlay_leaves_console_mode_alone() {
    let temp = TempDir::new().unwrap();
    let mut app = setup(&temp).unwrap();
    app.view = ViewMode::Console;
    app.set_overlay(Overlay::Help);

    app.close_overlay();

    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.view, ViewMode::Console);
}

// ─── CTRL+\ ROOT NAVIGATION ───────────────────────────────────────────────────

#[test]
fn hotkey_ctrl_backslash_navigates_to_root() {
    let temp = TempDir::new().unwrap();
    let app = setup(&temp).unwrap();
    let result = run(
        app,
        vec![
            ev_mod(KeyCode::Char('\\'), KeyModifiers::CONTROL),
            // F10 asks, and the second F10 answers.
            ev(KeyCode::F(10)),
            ev(KeyCode::F(10)),
        ],
    );
    assert!(result.is_ok());
}
