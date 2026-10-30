//! Unit tests: Ctrl+N flips the active panel only, says which form it shows,
//! and the short forms of a real listing are what Windows would give.

use super::toggle;
use crate::models::{App, Panel, PanelSide, ShortName, ShortNames};

fn listing(names: &[&str]) -> (tempfile::TempDir, App) {
    let dir = tempfile::tempdir().expect("temp dir");
    for name in names {
        std::fs::write(dir.path().join(name), b"").expect("touch");
    }
    let panel = Panel::new(dir.path().to_owned()).expect("a readable directory");
    (dir, App::from_panels(panel.clone(), panel))
}

#[test]
fn toggle_flips_only_the_active_panel_and_reports() {
    let (_dir, mut app) = listing(&["a.txt"]);
    app.active_panel = PanelSide::Right;
    assert!(
        !app.right_panel.options.short_names,
        "Far starts with long names"
    );

    toggle(&mut app);
    assert!(app.right_panel.options.short_names);
    assert!(
        !app.left_panel.options.short_names,
        "the other panel is untouched"
    );
    assert!(app.status.to_string().contains("Short names"));

    toggle(&mut app);
    assert!(!app.right_panel.options.short_names);
    assert!(app.status.to_string().contains("Long names"));
}

#[test]
fn the_menu_tick_follows_the_toggle() {
    use crate::menu::{Heading, Menu};
    use ratatui::{backend::TestBackend, Terminal};

    let (_dir, mut app) = listing(&["a.txt"]);
    let menu_row = |app: &mut App| {
        app.set_menu(Menu::open(Heading::Left));
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("test terminal");
        terminal.draw(|f| crate::ui::ui(f, app)).expect("draw");
        let buffer = terminal.backend().buffer();
        let row = (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .find(|line| line.contains("Show long names"))
            .expect("the Left menu has a Show long names row");
        app.close_overlay();
        row
    };
    assert!(menu_row(&mut app).contains("√ Show long names"));
    toggle(&mut app);
    assert!(!menu_row(&mut app).contains('√'));
}

#[test]
fn a_real_listing_gets_windows_style_short_names() {
    let (_dir, app) = listing(&[
        "longnamed.ext",
        "long name two.ext",
        "short.txt",
        "Makefile",
    ]);
    let panel = app.get_active_panel_ref();
    let table = ShortNames::for_entries(&panel.entries);
    let by_name = |name: &str| table.get(name).map(ShortName::as_str).map(str::to_owned);
    // Sorted by name, the blank sorts first: it takes `~1`.
    assert_eq!(
        by_name("long name two.ext").as_deref(),
        Some("LONGNA~1.EXT")
    );
    assert_eq!(by_name("longnamed.ext").as_deref(), Some("LONGNA~2.EXT"));
    assert_eq!(by_name("short.txt").as_deref(), Some("SHORT.TXT"));
    assert_eq!(by_name("Makefile").as_deref(), Some("MAKEFILE"));
    assert_eq!(by_name("..").as_deref(), Some(".."));
}
