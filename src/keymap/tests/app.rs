//! Purpose: a new `App` holds the default keymap, so that no test reads
//! the `ffm.yml` of the person running it.

use super::*;
use crate::models::{App, Panel};

#[test]
fn a_new_app_holds_the_default_keys() {
    let root = tempfile::tempdir().expect("a temporary folder");
    let panel = || Panel::new(root.path().to_path_buf()).expect("the folder lists");
    let app = App::with_panels(panel(), panel());
    assert_eq!(app.keymap, Keymap::default());
    assert_eq!(app.keymap.panels.keys(Action::View).count(), 1);
}
