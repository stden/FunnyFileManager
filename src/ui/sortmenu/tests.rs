//! Unit tests: the Sort by menu draws Far's rows, ticks the panel's mode and
//! order, highlights the cursor row, and records where it was drawn.

use ratatui::layout::Rect;
use ratatui::Terminal;

use super::{place, TICK};
use crate::models::{App, Overlay, PanelSide, SortMenu, SortMode, SortOrder, SortRow};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::{palette, render_overlays};

fn terminal() -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal")
}

fn draw(menu: SortMenu) -> (App, String, Terminal<WritableTestBackend>) {
    let mut terminal = terminal();
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::SortModes(menu));
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    let screen = buffer_text(&terminal);
    (app, screen, terminal)
}

/// The screen row carrying `label`, with its leading blanks kept.
fn row_with<'a>(screen: &'a str, label: &str) -> &'a str {
    screen
        .lines()
        .find(|line| line.contains(label))
        .unwrap_or_else(|| panic!("{label:?} is drawn:\n{screen}"))
}

#[test]
fn every_row_is_drawn_with_its_key_under_fars_title() {
    let (_, screen, _) = draw(SortMenu::open(
        SortMode::Name,
        SortOrder::Direct,
        PanelSide::Left,
    ));
    assert!(screen.contains("Sort by"), "{screen}");
    assert!(!screen.contains("Coming in this release"), "{screen}");
    for row in SortRow::ALL {
        let line = row_with(&screen, row.label());
        assert!(line.contains(&row.shortcut()), "{line:?}");
    }
    let name_at = screen.find("Name").expect("drawn");
    let reverse_at = screen.find("Reverse order").expect("drawn");
    assert!(name_at < reverse_at, "the order switch is the last row");
}

#[test]
fn the_current_mode_carries_the_tick_and_no_other_does() {
    let (_, screen, _) = draw(SortMenu::open(
        SortMode::Size,
        SortOrder::Direct,
        PanelSide::Left,
    ));
    let ticked: Vec<&str> = screen.lines().filter(|line| line.contains(TICK)).collect();
    assert_eq!(ticked.len(), 1, "{screen}");
    assert!(ticked[0].contains("Size"), "{}", ticked[0]);
}

#[test]
fn a_reversed_panel_ticks_the_reverse_row_as_well() {
    let (_, screen, _) = draw(SortMenu::open(
        SortMode::Extension,
        SortOrder::Reversed,
        PanelSide::Left,
    ));
    assert!(row_with(&screen, "Ext").contains(TICK));
    assert!(row_with(&screen, "Reverse order").contains(TICK));
    assert!(!row_with(&screen, "Owner").contains(TICK));
}

/// A rule separates the modes from the order switch, joined to the frame.
#[test]
fn a_rule_sits_between_the_modes_and_the_order_switch() {
    let (_, screen, _) = draw(SortMenu::open(
        SortMode::Name,
        SortOrder::Direct,
        PanelSide::Left,
    ));
    let lines: Vec<&str> = screen.lines().collect();
    let owner = lines
        .iter()
        .position(|line| line.contains("Owner"))
        .expect("drawn");
    let reverse = lines
        .iter()
        .position(|line| line.contains("Reverse order"))
        .expect("drawn");
    assert_eq!(reverse, owner + 2, "one rule row between them");
    let rule = lines[owner + 1];
    assert!(rule.contains("╟") && rule.contains("╢"), "{rule:?}");
}

#[test]
fn the_cursor_row_is_inverted_and_the_others_are_not() {
    let mut menu = SortMenu::open(SortMode::Name, SortOrder::Direct, PanelSide::Left);
    menu.down();
    let (_, screen, terminal) = draw(menu);
    let buffer = terminal.backend().buffer();
    let lines: Vec<&str> = screen.lines().collect();
    let ext_y = lines
        .iter()
        .position(|line| line.contains("Ext"))
        .expect("drawn");
    let ext_x = lines[ext_y].find("Ext").expect("drawn");
    let cell = |x: usize, y: usize| {
        &buffer[(
            u16::try_from(x).expect("fits"),
            u16::try_from(y).expect("fits"),
        )]
    };
    assert_eq!(cell(ext_x, ext_y).bg, palette::MENU_SELECTED_BG);
    let name_y = lines
        .iter()
        .position(|line| line.contains("Name"))
        .expect("drawn");
    assert_eq!(cell(ext_x, name_y).bg, palette::MENU_BG);
}

#[test]
fn the_frame_is_recorded_for_the_mouse_and_follows_a_drag() {
    let (app, _, _) = draw(SortMenu::default());
    let centred = app.dialog_mouse.area.expect("the frame was tracked");

    let mut terminal = terminal();
    let mut dragged = App::new().expect("test app");
    dragged.set_overlay(Overlay::SortModes(SortMenu::default()));
    dragged.dialog_mouse.offset = (5, 2);
    terminal
        .draw(|f| render_overlays(f, &mut dragged))
        .expect("draw");
    let moved = dragged.dialog_mouse.area.expect("tracked");
    assert_eq!((moved.x, moved.y), (centred.x + 5, centred.y + 2));
    assert_eq!((moved.width, moved.height), (centred.width, centred.height));
}

/// The box is sized to its rows: every row fits, nothing is clipped, and
/// the screen has room to spare around it.
#[test]
fn the_box_is_sized_to_its_content() {
    let area = place(Rect::new(0, 0, 80, 24), PanelSide::Left, (0, 0)).expect("fits");
    assert_eq!(
        usize::from(area.height),
        SortRow::ALL.len() + 3,
        "rows, the rule, and the frame"
    );
    assert!(area.width < 40, "{area:?}");
    assert!(area.x > 0 && area.y > 0, "centred: {area:?}");
}

/// A screen too small for even the frame gets nothing rather than a panic or
/// a box hanging off the edge; a drag cannot push the box off screen either.
#[test]
fn a_tiny_screen_and_a_far_drag_are_both_safe() {
    assert!(place(Rect::new(0, 0, 3, 2), PanelSide::Left, (0, 0)).is_none());
    let screen = Rect::new(0, 0, 80, 24);
    let pushed = place(screen, PanelSide::Left, (1000, 1000)).expect("fits");
    assert!(pushed.right() <= screen.right() && pushed.bottom() <= screen.bottom());
    let pulled = place(screen, PanelSide::Left, (-1000, -1000)).expect("fits");
    assert_eq!((pulled.x, pulled.y), (0, 0));

    let mut terminal = Terminal::new(WritableTestBackend::new(3, 2)).expect("test terminal");
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::SortModes(SortMenu::default()));
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    assert!(app.dialog_mouse.area.is_none(), "nothing to click on");
}
