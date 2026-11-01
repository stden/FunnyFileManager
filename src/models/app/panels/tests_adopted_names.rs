//! Purpose: A panel whose folder cannot be re-read when the description names
//! change keeps its last listing, and still takes the new names.

use super::*;
use tempfile::TempDir;

#[test]
fn a_panel_whose_folder_vanished_keeps_its_listing_and_takes_the_names() {
    let root = TempDir::new().unwrap();
    let gone = root.path().join("gone");
    std::fs::create_dir(&gone).unwrap();
    std::fs::write(gone.join("kept.txt"), "").unwrap();
    let left = Panel::new(gone.clone()).unwrap();
    let right = Panel::new(root.path().to_path_buf()).unwrap();
    let mut app = App::with_panels(left, right);
    let listed = app.left_panel.entries.len();
    std::fs::remove_dir_all(&gone).unwrap();
    let names = vec!["notes.txt".to_owned()];
    app.config.descriptions.names.clone_from(&names);

    app.adopt_description_names();

    assert_eq!(app.left_panel.description_names, names);
    assert_eq!(
        app.left_panel.entries.len(),
        listed,
        "the last listing stays"
    );
}
