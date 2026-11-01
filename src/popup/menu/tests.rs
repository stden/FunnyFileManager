//! Purpose: Alt heading shortcuts must take priority over drop-down row shortcuts.

use super::*;
use crate::{
    menu::{Heading, Menu},
    models::Panel,
};

#[test]
fn alt_letters_select_every_heading_from_every_drop_down() {
    let dir = tempfile::TempDir::new().expect("fixture");
    let panel = Panel::from_entries(dir.path().to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    for source in Heading::ALL {
        for (letter, heading) in [
            ('l', Heading::Left),
            ('f', Heading::Files),
            ('c', Heading::Commands),
            ('o', Heading::Options),
            ('r', Heading::Right),
        ] {
            for ch in [letter, letter.to_ascii_uppercase()] {
                app.set_menu(Menu::open(source));
                assert!(process(&mut app, KeyCode::Char(ch), KeyModifiers::ALT));
                assert!(
                    matches!(&app.overlay, Overlay::Menu(menu) if menu.heading == heading),
                    "Alt+{ch} from {source:?} must select {heading:?}"
                );
                assert!(app.pending_row.is_none(), "no row may execute");
            }
        }
    }
}
