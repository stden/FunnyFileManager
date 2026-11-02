//! Purpose: Unit tests for file-association lookup and dialog opening.

use super::{association_command, open};
use crate::models::{App, Overlay, Panel};

#[test]
fn association_command_returns_none_and_open_mounts_overlay() {
    let temp = tempfile::TempDir::new().unwrap();
    let left = Panel::new(temp.path().to_path_buf()).unwrap();
    let right = Panel::new(temp.path().to_path_buf()).unwrap();
    let mut app = App::with_panels(left, right);
    assert_eq!(association_command(&app), None);
    open(&mut app);
    assert!(matches!(app.overlay, Overlay::Associations(_)));
}
