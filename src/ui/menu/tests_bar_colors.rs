//! Purpose: Pin exact Far Manager menu bar and drop-down colors, padding, and hotkeys.

use ratatui::style::Modifier;

use super::geometry::drop_down_area;
use super::*;
use crate::menu::Menu;

fn draw_right_menu() -> ratatui::buffer::Buffer {
    let temp = tempfile::TempDir::new().unwrap();
    let panel = crate::models::Panel::new(temp.path().to_owned()).unwrap();
    let mut app = App::from_panels(panel.clone(), panel);
    app.set_menu(Menu::open(Heading::Right));

    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
    terminal
        .draw(|frame| crate::ui::ui(frame, &mut app))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn find_row_symbol<'a>(
    buffer: &'a ratatui::buffer::Buffer,
    area: Rect,
    row_offset: u16,
    ch: &str,
) -> &'a ratatui::buffer::Cell {
    let y = area.y + row_offset;
    (area.x + 1..area.right() - 1)
        .map(|x| &buffer[(x, y)])
        .find(|cell| cell.symbol() == ch)
        .unwrap_or_else(|| panic!("missing {ch:?} on row offset {row_offset}"))
}

#[test]
fn top_menu_bar_matches_far_teal_black_yellow_and_white_scheme() {
    let buffer = draw_right_menu();
    assert_eq!(buffer[(0, 0)].bg, palette::MENU_BAR_BG);
    assert_eq!(buffer[(70, 0)].bg, palette::MENU_BAR_BG);

    for (x, ch) in [(4, "L"), (12, "F"), (21, "C"), (33, "O")] {
        assert_eq!(buffer[(x, 0)].symbol(), ch);
        assert_eq!(buffer[(x, 0)].fg, palette::HOTKEY);
        assert_eq!(buffer[(x, 0)].bg, palette::MENU_BAR_BG);
        assert!(!buffer[(x, 0)].modifier.contains(Modifier::UNDERLINED));
    }
    for (x, ch) in [(5, "e"), (13, "i"), (22, "o"), (34, "p")] {
        assert_eq!(buffer[(x, 0)].symbol(), ch);
        assert_eq!(buffer[(x, 0)].fg, palette::MENU_FG);
        assert_eq!(buffer[(x, 0)].bg, palette::MENU_BAR_BG);
    }

    assert_eq!(buffer[(41, 0)].bg, palette::MENU_BAR_BG);
    for x in 42..51 {
        assert_eq!(buffer[(x, 0)].bg, palette::MENU_SELECTED_BG, "col {x}");
    }
    assert_eq!(buffer[(44, 0)].fg, palette::HOTKEY);
    assert!(!buffer[(44, 0)].modifier.contains(Modifier::UNDERLINED));
    assert_eq!(buffer[(45, 0)].fg, palette::MENU_SELECTED_FG);
    assert_eq!(buffer[(51, 0)].bg, palette::MENU_BAR_BG);
}

#[test]
fn dropdown_matches_far_teal_white_border_and_yellow_hotkeys() {
    let buffer = draw_right_menu();
    let area = drop_down_area(
        &Menu::open(Heading::Right),
        Rect::new(0, 0, 80, 1),
        Rect::new(0, 0, 80, 24),
    )
    .unwrap();
    for (x, y, sym) in [
        (area.x, area.y, "╔"),
        (area.x, area.y + 11, "╟"),
        (area.x + 1, area.y + 11, "─"),
    ] {
        assert_eq!(buffer[(x, y)].symbol(), sym);
        assert_eq!(buffer[(x, y)].fg, palette::MENU_SELECTED_FG);
        assert_eq!(buffer[(x, y)].bg, palette::MENU_BAR_BG);
    }
    let b_key = find_row_symbol(&buffer, area, 1, "B");
    assert_eq!(b_key.bg, palette::MENU_SELECTED_BG);
    assert_eq!(b_key.fg, palette::HOTKEY);
    assert!(!b_key
        .modifier
        .intersects(Modifier::UNDERLINED | Modifier::BOLD));
    for sym in ["r", "C"] {
        let cell = find_row_symbol(&buffer, area, 1, sym);
        assert_eq!(cell.bg, palette::MENU_SELECTED_BG);
        assert_eq!(cell.fg, palette::MENU_SELECTED_FG);
    }
}

#[test]
fn dropdown_unselected_rows_use_far_teal_white_and_plain_yellow_hotkeys() {
    let buffer = draw_right_menu();
    let area = drop_down_area(
        &Menu::open(Heading::Right),
        Rect::new(0, 0, 80, 1),
        Rect::new(0, 0, 80, 24),
    )
    .unwrap();
    for (row_offset, key, plain) in [(2, "M", "e"), (3, "F", "√")] {
        let hotkey = find_row_symbol(&buffer, area, row_offset, key);
        assert_eq!(hotkey.bg, palette::MENU_BAR_BG);
        assert_eq!(hotkey.fg, palette::HOTKEY);
        assert!(!hotkey
            .modifier
            .intersects(Modifier::UNDERLINED | Modifier::BOLD));
        let normal = find_row_symbol(&buffer, area, row_offset, plain);
        assert_eq!(normal.bg, palette::MENU_BAR_BG);
        assert_eq!(normal.fg, palette::MENU_SELECTED_FG);
    }
}

#[test]
fn heading_at_hits_padded_headings_and_bar_hotkeys_handle_markers() {
    let bar = Rect::new(0, 0, 80, 1);
    assert_eq!(heading_at(bar, 1), None);
    assert_eq!(heading_at(bar, 2), Some(Heading::Left));
    assert_eq!(heading_at(bar, 9), Some(Heading::Left));
    assert_eq!(heading_at(bar, 42), Some(Heading::Right));
    assert_eq!(heading_at(bar, 50), Some(Heading::Right));
    assert_eq!(heading_at(bar, 51), None);

    let plain: String = bar_hotkey_spans("Plain", Style::default())
        .into_iter()
        .map(|s| s.content.into_owned())
        .collect();
    assert_eq!(plain, "Plain");

    let dangling: String = bar_hotkey_spans("Trail&", Style::default())
        .into_iter()
        .map(|s| s.content.into_owned())
        .collect();
    assert_eq!(dangling, "Trail");
}
