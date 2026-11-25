use super::{close_list, step_focus, step_list, toggle_enabled};
use crate::models::lsp_editor::Direction;
use crate::models::{App, Overlay, Panel};
use tempfile::TempDir;

#[test]
fn form_helpers_do_nothing_when_lsp_editor_is_not_open() {
    let dir = TempDir::new().expect("temp");
    let panel = Panel::from_entries(dir.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);

    step_focus(&mut app, Direction::Next);
    toggle_enabled(&mut app);
    step_list(&mut app, Direction::Next);
    close_list(&mut app);

    assert!(matches!(app.overlay, Overlay::None));
}
