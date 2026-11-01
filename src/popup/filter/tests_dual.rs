//! Distinct active include masks remain visible and independently editable.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::mask::Mask;
use crate::models::{App, Panel};
use crate::popup::process_popup_input;

#[test]
fn a_second_include_can_be_cleared_without_losing_the_first() {
    let temp = tempfile::TempDir::new().expect("temp");
    for name in ["x.rs", "y.rs", "x.txt"] {
        std::fs::write(temp.path().join(name), "test").expect("file");
    }
    let panel = Panel::new(temp.path().to_owned()).expect("panel");
    let mut app = App::from_panels(panel.clone(), panel);
    app.left_panel.options.filter = "*.rs".to_owned();
    app.left_panel.filter.include = Some(Mask::parse("*x*"));
    app.left_panel.load_entries().expect("filter listing");
    crate::ops::filter::open(&mut app);

    process_popup_input(&mut app, KeyCode::Tab.into());
    process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
    );
    process_popup_input(&mut app, KeyCode::Backspace.into());
    process_popup_input(&mut app, KeyCode::Enter.into());

    assert_eq!(app.left_panel.options.filter, "*.rs");
    assert!(app.left_panel.filter.include.is_none());
    let names: Vec<_> = app
        .left_panel
        .entries
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert!(names.contains(&"x.rs"));
    assert!(names.contains(&"y.rs"));
    assert!(!names.contains(&"x.txt"));
}
