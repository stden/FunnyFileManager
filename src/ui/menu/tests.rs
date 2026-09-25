//! Unit tests: the drop-down's Far colours and its shadow, labels that drop
//! their hotkey markers, and the gap every shortcut keeps after its label.
//! Where a drop-down goes, and what a click on it hits, is in
//! `tests_geometry`.

use super::geometry::drop_down_area;
use super::*;
use crate::menu::{Entry, Menu};

pub(super) fn screen(width: u16, height: u16) -> Rect {
    Rect::new(0, 0, width, height)
}

pub(super) fn bar(width: u16) -> Rect {
    Rect::new(0, 0, width, 1)
}

#[test]
fn dropdown_keeps_far_colors_when_panel_highlighting_is_disabled() {
    let temp = tempfile::TempDir::new().unwrap();
    let panel = crate::models::Panel::new(temp.path().to_owned()).unwrap();
    let mut app = App::from_panels(panel.clone(), panel);
    app.settings
        .set(crate::menu::SettingsPage::Highlighting, "off")
        .unwrap();
    let menu = Menu::open(Heading::Files);
    app.set_menu(menu);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    terminal
        .draw(|frame| crate::ui::ui(frame, &mut app))
        .unwrap();
    let area = drop_down_area(&menu, bar(120), screen(120, 40)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(area.x, area.y)].fg, palette::MENU_SELECTED_FG);
    assert_eq!(buffer[(area.x, area.y)].bg, palette::MENU_BG);
    assert_eq!(
        buffer[(area.x + 1, area.y + 1)].fg,
        palette::MENU_SELECTED_FG
    );
    assert_eq!(
        buffer[(area.x + 1, area.y + 1)].bg,
        palette::MENU_SELECTED_BG
    );
    assert_eq!(
        buffer[(area.x + 1, area.y + 2)].fg,
        palette::MENU_SELECTED_FG
    );
    assert_eq!(buffer[(area.x + 1, area.y + 2)].bg, palette::MENU_BG);
}

#[test]
fn dropdown_casts_a_shadow_over_the_existing_background() {
    let temp = tempfile::TempDir::new().unwrap();
    let panel = crate::models::Panel::new(temp.path().to_owned()).unwrap();
    let mut app = App::from_panels(panel.clone(), panel);
    let menu = Menu::open(Heading::Files);
    app.set_menu(menu);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    terminal
        .draw(|frame| {
            frame.render_widget(
                Paragraph::new("background")
                    .style(Style::default().bg(ratatui::style::Color::Rgb(80, 80, 80))),
                frame.area(),
            );
            render_drop_down(frame, &app, bar(120));
        })
        .unwrap();
    let area = drop_down_area(&menu, bar(120), screen(120, 40)).unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(area.right(), area.y + 1)].bg,
        ratatui::style::Color::Rgb(30, 30, 30)
    );
    assert_eq!(
        buffer[(area.right() + 2, area.y + 1)].bg,
        ratatui::style::Color::Rgb(80, 80, 80)
    );
}

// ─── labels ─────────────────────────────────────────────────────────────────

/// The text of a label's spans, joined back up. A renderer that dropped or
/// duplicated a character would show it here.
fn drawn(label: &str) -> String {
    hotkey_spans_in(label, Style::default(), true)
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

#[test]
fn a_label_with_no_marker_is_drawn_whole() {
    assert_eq!(drawn("No marker here"), "No marker here");
    assert_eq!(
        hotkey_spans_in("No marker", Style::default(), true).len(),
        1
    );
}

/// A marker with nothing after it keeps the text before it rather than
/// panicking on the missing letter.
#[test]
fn a_dangling_marker_is_dropped_and_the_rest_survives() {
    assert_eq!(drawn("Dangling &"), "Dangling ");
}

#[test]
fn a_marked_label_loses_the_marker_and_keeps_every_letter() {
    assert_eq!(drawn("&Delete"), "Delete");
    assert_eq!(drawn("Find pre&vious"), "Find previous");
    assert!(
        hotkey_spans_in("&Delete", Style::default(), true).len() > 1,
        "the hotkey letter is its own span, so it can be coloured"
    );
}

/// Unaccented is how the selected row is drawn: the same characters, one
/// span fewer worth of colour.
#[test]
fn an_unaccented_label_still_reads_the_same() {
    let accented = drawn("&Delete");
    let plain: String = hotkey_spans_in("&Delete", Style::default(), false)
        .iter()
        .map(|span| span.content.as_ref())
        .collect();
    assert_eq!(plain, accented);
}

// ─── shortcuts ──────────────────────────────────────────────────────────────

/// The widest row of a drop-down used to print its label flush against its
/// shortcut — "Long descriptionsCtrl+7", "Archive commandsShift+F3" — because
/// the box was sized with no room for a gap. Every row keeps two blanks.
#[test]
fn every_shortcut_keeps_a_gap_after_its_label() {
    let temp = tempfile::TempDir::new().unwrap();
    let panel = crate::models::Panel::new(temp.path().to_owned()).unwrap();
    for heading in Heading::ALL {
        let mut app = App::from_panels(panel.clone(), panel.clone());
        let menu = Menu::open(heading);
        app.set_menu(menu);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
        terminal
            .draw(|frame| crate::ui::ui(frame, &mut app))
            .unwrap();
        let area = drop_down_area(&menu, bar(120), screen(120, 40)).unwrap();
        let buffer = terminal.backend().buffer();
        for (row, entry) in (area.y + 1..).zip(menu.entries()) {
            let Entry::Item { shortcut, .. } = entry else {
                continue;
            };
            if shortcut.is_empty() {
                continue;
            }
            let text: String = (area.x + 1..area.right() - 1)
                .map(|x| buffer[(x, row)].symbol())
                .collect();
            assert!(
                text.trim_end().ends_with(&format!("  {shortcut}")),
                "{heading:?}: {text:?}"
            );
        }
    }
}

/// At 40 columns the headings fill the bar, and the clock was drawn over
/// them: `   Left   Files   Commands   Op 3:04 PMR`. It gives way now; at 80
/// columns there is room for both.
#[test]
fn the_clock_never_covers_a_heading() {
    let headings = "   Left   Files   Commands   Options   Right";
    let temp = tempfile::TempDir::new().unwrap();
    let panel = crate::models::Panel::new(temp.path().to_owned()).unwrap();
    for width in [40_u16, 80] {
        let mut app = App::from_panels(panel.clone(), panel.clone());
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, 12)).unwrap();
        terminal
            .draw(|frame| crate::ui::ui(frame, &mut app))
            .unwrap();
        let row: String = (0..width)
            .map(|x| terminal.backend().buffer()[(x, 0)].symbol().to_owned())
            .collect();
        let shown: String = headings.chars().take(usize::from(width)).collect();
        assert!(row.starts_with(&shown), "{width}: {row:?}");
        assert_eq!(
            row.contains(':'),
            width == 80,
            "the clock at {width}: {row:?}"
        );
    }
}
