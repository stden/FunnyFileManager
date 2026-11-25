//! Purpose: The Commands menu row opens the dialog.

use crate::command::language::MenuCommand;
use crate::models::{App, Overlay, Panel};

#[test]
fn the_commands_menu_row_opens_the_dialog() {
    let temp = tempfile::TempDir::new().expect("temp");
    let list = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    let mut app = App::with_panels(list.clone(), list);
    crate::ops::menu::execute(&mut app, MenuCommand::CloudflareDns);
    assert!(matches!(app.overlay, Overlay::CloudflareDns(_)));
}
