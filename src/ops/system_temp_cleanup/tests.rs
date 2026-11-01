//! Purpose: Default-No confirmation is mandatory before a deletion can be queued.
use super::*;
use crate::models::{temp_cleanup::TempSource, Panel};
#[test]
fn opening_and_cancel_do_not_delete_fixture_files() {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::write(temp.path().join("keep"), b"keep").unwrap();
    let panel = Panel::from_entries(temp.path().to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.live.temp_cleanup = TempSource::Directory {
        path: temp.path().to_owned(),
        home: None,
    };
    open(&mut app);
    assert!(matches!(app.overlay, Overlay::TempCleanup(_)));
    cancel(&mut app);
    assert_eq!(std::fs::read(temp.path().join("keep")).unwrap(), b"keep");
}
