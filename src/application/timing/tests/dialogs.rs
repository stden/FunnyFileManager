//! Purpose: The database explorer and the duplicates dialog have names of their
//! own in the statistics, and a pasted text is never recorded in them.

use super::*;
use crate::models::database::DatabaseExplorer;
use crate::models::Duplicates;

#[test]
fn the_database_explorer_and_duplicates_dialog_label_their_own_actions() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let mut app = app(&temp);
    for (overlay, expected) in [
        (
            Overlay::Database(Box::new(DatabaseExplorer::new(Vec::new(), None))),
            "Database",
        ),
        (
            Overlay::Duplicates(Box::new(Duplicates::new("/private/project"))),
            "Duplicates",
        ),
    ] {
        app.set_overlay(overlay);
        let key = Event::Key(KeyEvent::new(
            KeyCode::F(5),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(
            label(&app, &key),
            format!("{expected}/KeyModifiers(0x0)/F(5)")
        );
        assert_eq!(
            label(&app, &Event::Paste("private payload".into())),
            format!("{expected}/Paste")
        );
    }
}
