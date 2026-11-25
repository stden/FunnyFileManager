use super::execute;
use crate::action::tasks::TaskCommand;
use crate::models::{App, Overlay, Panel};
use tempfile::TempDir;

#[test]
fn execute_is_inert_when_task_list_is_not_open() {
    let dir = TempDir::new().expect("temp");
    let panel = Panel::from_entries(dir.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);

    execute(&mut app, TaskCommand::Help);

    assert!(matches!(app.overlay, Overlay::None));
}
