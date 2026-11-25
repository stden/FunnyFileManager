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
    app.set_menu(Menu::open(Heading::Left));
    assert!(process(&mut app, KeyCode::Enter, KeyModifiers::ALT));
    assert!(matches!(&app.overlay, Overlay::Menu(menu) if menu.heading == Heading::Left));
}

#[test]
fn alt_with_an_unbound_heading_keeps_the_existing_menu_open() {
    let panel = Panel::from_entries(std::env::temp_dir(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    app.set_menu(Menu::open(Heading::Files));
    assert!(process(&mut app, KeyCode::Char('z'), KeyModifiers::ALT));
    assert!(matches!(&app.overlay, Overlay::Menu(menu) if menu.heading == Heading::Files));
    assert!(app.pending_row.is_none());
}

#[test]
fn an_alt_navigation_key_keeps_the_drop_down_and_its_selection() {
    let panel = Panel::from_entries(std::env::temp_dir(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let menu = Menu::open(Heading::Files);
    app.set_menu(menu);
    assert!(process(&mut app, KeyCode::Right, KeyModifiers::ALT));
    assert!(matches!(app.overlay, Overlay::Menu(current) if current == menu));
    assert!(app.pending_row.is_none());
}
