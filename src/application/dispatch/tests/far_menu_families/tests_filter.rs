//! The menu exposes filtering even when Ctrl+I is encoded as Tab.

use super::*;

fn press(terminal: &mut Terminal<WritableTestBackend>, app: &mut App, code: KeyCode) {
    dispatch_key(terminal, app, key_event(code));
}

fn open_filter(terminal: &mut Terminal<WritableTestBackend>, app: &mut App) {
    for code in [
        KeyCode::F(9),
        KeyCode::Right,
        KeyCode::Right,
        KeyCode::Char('i'),
    ] {
        press(terminal, app, code);
    }
}

#[test]
fn f9_filter_applies_reopens_and_tab_still_switches_panels() {
    let dir = tempfile::TempDir::new().expect("temp");
    std::fs::write(dir.path().join("keep.rs"), "test").expect("file");
    std::fs::write(dir.path().join("omit.txt"), "test").expect("file");
    std::fs::create_dir(dir.path().join("docs")).expect("folder");
    let panel = models::Panel::new(dir.path().to_owned()).expect("panel");
    let mut app = App::from_panels(panel.clone(), panel);
    let mut term = terminal();
    open_filter(&mut term, &mut app);
    assert!(matches!(app.overlay, Overlay::Filter(_)), "filter dialog");
    for ch in "*.RS".chars() {
        press(&mut term, &mut app, KeyCode::Char(ch));
    }
    press(&mut term, &mut app, KeyCode::Enter);
    let names: Vec<_> = app
        .left_panel
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(names.contains(&"keep.rs"));
    assert!(names.contains(&"docs"));
    assert!(!names.contains(&"omit.txt"));
    open_filter(&mut term, &mut app);
    assert!(matches!(&app.overlay, Overlay::Filter(dialog) if dialog.include == "*.RS"));
    press(&mut term, &mut app, KeyCode::Esc);
    press(&mut term, &mut app, KeyCode::Tab);
    assert_eq!(app.active_panel, PanelSide::Right);
}
