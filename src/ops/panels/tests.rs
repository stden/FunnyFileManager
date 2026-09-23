//! Unit tests: `Ctrl+F1` / `Ctrl+F2` take a panel off the screen and give
//! its half to the other, the cursor never lands on a hidden panel, and the
//! last panel standing cannot be hidden.

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::Terminal;

use super::{select, swap_panels, switch_panel, toggle};
use crate::application::dispatch_key;
use crate::menu::{Entry, Heading, Menu};
use crate::models::{App, Overlay, PanelSide, PanelsShown};
use crate::mouse::{hit, Target};
use crate::test_support::{buffer_text, key_event, key_event_with_mods, WritableTestBackend};
use crate::ui::{checked_entry, ui, FKEY_COUNT};

fn app() -> App {
    App::new().expect("test app")
}

#[test]
fn toggle_hides_a_panel_and_reports_it() {
    let mut app = app();
    toggle(&mut app, PanelSide::Right);
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    assert_eq!(app.status.to_string(), "Right panel off");
}

#[test]
fn toggle_twice_brings_the_panel_back() {
    let mut app = app();
    toggle(&mut app, PanelSide::Left);
    toggle(&mut app, PanelSide::Left);
    assert_eq!(app.panels_shown, PanelsShown::Both);
    assert_eq!(app.status.to_string(), "Left panel on");
}

#[test]
fn hiding_the_active_panel_moves_the_cursor_to_the_other() {
    let mut app = app();
    app.active_panel = PanelSide::Left;
    toggle(&mut app, PanelSide::Left);
    assert_eq!(app.panels_shown, PanelsShown::RightOnly);
    assert_eq!(app.active_panel, PanelSide::Right);
}

#[test]
fn hiding_the_inactive_panel_keeps_the_cursor_where_it_was() {
    let mut app = app();
    app.active_panel = PanelSide::Left;
    toggle(&mut app, PanelSide::Right);
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    assert_eq!(app.active_panel, PanelSide::Left);
}

/// Far lets Ctrl+F1 then Ctrl+F2 leave no panel on screen (the console shows
/// through); this program keeps the last one, since the freed screen has no
/// console worth looking at, and says so instead of pretending the key did
/// something.
#[test]
fn the_last_panel_cannot_be_hidden() {
    let mut app = app();
    toggle(&mut app, PanelSide::Right);
    toggle(&mut app, PanelSide::Left);
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    assert_eq!(app.active_panel, PanelSide::Left);
    assert_eq!(
        app.status.to_string(),
        "Left panel stays on: the last panel cannot be hidden (Ctrl+O hides both)"
    );
}

/// Far ignores Tab while a panel is hidden: the cursor would otherwise sit on
/// files nobody can see.
#[test]
fn tab_does_not_reach_a_hidden_panel() {
    let mut app = app();
    app.active_panel = PanelSide::Left;
    toggle(&mut app, PanelSide::Right);
    switch_panel(&mut app);
    assert_eq!(app.active_panel, PanelSide::Left);
    toggle(&mut app, PanelSide::Right);
    switch_panel(&mut app);
    assert_eq!(
        app.active_panel,
        PanelSide::Right,
        "Tab works again once both are shown"
    );
}

#[test]
fn left_and_right_keys_select_only_a_visible_panel() {
    let mut app = app();
    app.active_panel = PanelSide::Right;
    toggle(&mut app, PanelSide::Left);
    select(&mut app, PanelSide::Left);
    assert_eq!(
        app.active_panel,
        PanelSide::Right,
        "the left panel is off screen"
    );
    select(&mut app, PanelSide::Right);
    assert_eq!(app.active_panel, PanelSide::Right);
    toggle(&mut app, PanelSide::Left);
    select(&mut app, PanelSide::Left);
    assert_eq!(app.active_panel, PanelSide::Left);
}

/// The menu's "Panel On/Off" tick follows the toggle for the heading's side.
#[test]
fn the_menu_tick_follows_the_toggle() {
    let mut app = app();
    toggle(&mut app, PanelSide::Right);
    assert!(!panel_on_ticked(&app, Heading::Right));
    assert!(panel_on_ticked(&app, Heading::Left));
    toggle(&mut app, PanelSide::Right);
    assert!(panel_on_ticked(&app, Heading::Right));
}

fn panel_on_ticked(app: &App, heading: Heading) -> bool {
    let entry = Menu::open(heading)
        .entries()
        .iter()
        .find(|entry| matches!(entry, Entry::Item { label, .. } if *label == "Panel &On/Off"))
        .expect("the Panel On/Off row");
    matches!(
        checked_entry(app, *entry),
        Entry::Item { checked: true, .. }
    )
}

/// A click on the half the hidden panel used to have lands on the panel
/// that now fills it, never on the hidden one.
#[test]
fn a_click_on_the_freed_half_lands_on_the_visible_panel() {
    let mut app = app();
    let screen = ratatui::layout::Rect::new(0, 0, 80, 24);
    toggle(&mut app, PanelSide::Left);
    let target = hit(screen, 10, 3, FKEY_COUNT, app.panels_on_screen());
    assert!(
        matches!(
            target,
            Target::Panel(PanelSide::Right)
                | Target::PanelRow {
                    side: PanelSide::Right,
                    ..
                }
        ),
        "{target:?}"
    );
}

/// Rendered through the real layout: the visible panel's title is centred on
/// the whole width, and the hidden panel's path is nowhere on screen.
#[test]
fn the_visible_panel_takes_the_full_width_on_screen() {
    let root = tempfile::tempdir().expect("temp dir");
    let (left_dir, right_dir) = two_directories(root.path());
    let mut app = app();
    app.left_panel.navigate_to(left_dir).expect("left panel");
    app.right_panel.navigate_to(right_dir).expect("right panel");

    let both = draw(&mut app);
    let title_row = both.lines().nth(1).expect("title row");
    let half_column = title_row.find("left_side_dir").expect("left title");
    assert!(half_column < 40, "centred in the left half: {title_row:?}");
    assert!(both.contains("right_side_dir"), "{both}");

    toggle(&mut app, PanelSide::Right);
    let one = draw(&mut app);
    let title_row = one.lines().nth(1).expect("title row");
    let full_column = title_row.find("left_side_dir").expect("left title");
    assert!(
        full_column > half_column,
        "centred on the full width: {title_row:?}"
    );
    assert!(
        title_row.trim_end().len() >= 79,
        "the frame spans the screen: {title_row:?}"
    );
    assert!(
        !one.contains("right_side_dir"),
        "the hidden panel is off screen: {one}"
    );
}

/// Two distinctly named directories, one per panel, so a title on screen
/// says which panel is drawn.
fn two_directories(root: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let left_dir = root.join("left_side_dir");
    let right_dir = root.join("right_side_dir");
    std::fs::create_dir_all(&left_dir).expect("left dir");
    std::fs::create_dir_all(&right_dir).expect("right dir");
    (left_dir, right_dir)
}

fn draw(app: &mut App) -> String {
    let mut terminal = terminal();
    terminal.draw(|f| ui(f, app)).expect("draw");
    buffer_text(&terminal)
}

fn terminal() -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal")
}

/// The keys themselves, through the dispatcher: `Ctrl+F2` hides the right
/// panel, and `Tab` / `Right` then leave the cursor on the left one.
#[test]
fn ctrl_f2_then_tab_and_right_keep_the_cursor_on_the_left_panel() {
    let mut app = app();
    let mut terminal = terminal();
    app.active_panel = PanelSide::Left;
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::F(2), KeyModifiers::CONTROL),
    );
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Tab));
    assert_eq!(app.active_panel, PanelSide::Left, "Tab is ignored");
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Right));
    assert_eq!(app.active_panel, PanelSide::Left, "Right is ignored");
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::F(2), KeyModifiers::CONTROL),
    );
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Tab));
    assert_eq!(
        app.active_panel,
        PanelSide::Right,
        "Tab works once both are shown"
    );
}

/// `Ctrl+F1` on the active left panel: the right one takes the cursor.
#[test]
fn ctrl_f1_on_the_active_panel_hands_the_cursor_over() {
    let mut app = app();
    let mut terminal = terminal();
    app.active_panel = PanelSide::Left;
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::F(1), KeyModifiers::CONTROL),
    );
    assert_eq!(app.panels_shown, PanelsShown::RightOnly);
    assert_eq!(app.active_panel, PanelSide::Right);
}

/// The F9 menu row is the same toggle as the key: choosing "Panel On/Off"
/// under the Right heading hides the right panel, and the row's tick follows.
#[test]
fn the_menu_row_is_the_same_toggle_as_the_key() {
    let mut app = app();
    crate::ops::menu::execute(
        &mut app,
        crate::menu::MenuCommand::TogglePanel(PanelSide::Right),
    );
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    assert_eq!(app.status.to_string(), "Right panel off");
    assert!(!panel_on_ticked(&app, Heading::Right));
}

/// `F9` then `Left` / `Right` walks the bar over the Left and Right headings.
/// Walking never moves the cursor to another panel; when the panel is hidden
/// its drop-down still opens (its "Panel On/Off" row is how the panel comes
/// back) and the cursor stays on the panel that is on screen.
#[test]
fn the_f9_bar_never_walks_the_cursor_onto_a_hidden_panel() {
    let mut app = app();
    let mut terminal = terminal();
    app.active_panel = PanelSide::Left;
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::F(2), KeyModifiers::CONTROL),
    );
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::F(9)));
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Left));
    assert!(on_right_heading(&app), "the bar wraps onto Right");
    assert_eq!(app.active_panel, PanelSide::Left, "wrapping onto Right");
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Esc));

    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::F(9)));
    for _ in 0..4 {
        dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Right));
    }
    assert!(
        on_right_heading(&app),
        "Left, Files, Commands, Options, Right"
    );
    assert_eq!(app.active_panel, PanelSide::Left, "walking onto Right");
    dispatch_key(&mut terminal, &mut app, key_event(KeyCode::Esc));
    assert_eq!(app.active_panel, PanelSide::Left);
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
}

fn on_right_heading(app: &App) -> bool {
    matches!(app.overlay, Overlay::Menu(menu) if menu.heading == Heading::Right)
}

/// Far's `SwapPanels` does nothing unless both panels are visible: swapping
/// would put the hidden panel's directory on screen under the other's name.
#[test]
fn ctrl_u_is_ignored_while_a_panel_is_hidden() {
    let root = tempfile::tempdir().expect("temp dir");
    let (left_dir, right_dir) = two_directories(root.path());
    let mut app = app();
    let mut terminal = terminal();
    app.left_panel
        .navigate_to(left_dir.clone())
        .expect("left panel");
    app.right_panel
        .navigate_to(right_dir.clone())
        .expect("right panel");
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::F(2), KeyModifiers::CONTROL),
    );
    dispatch_key(
        &mut terminal,
        &mut app,
        key_event_with_mods(KeyCode::Char('u'), KeyModifiers::CONTROL),
    );
    assert_eq!(app.left_panel.path, left_dir, "the swap is refused");
    assert_eq!(app.right_panel.path, right_dir);
    assert_eq!(app.panels_shown, PanelsShown::LeftOnly);
    assert_eq!(
        app.status.to_string(),
        "Panels are not swapped while one is hidden (Ctrl+F2 shows the right panel)"
    );
    assert!(!draw(&mut app).contains("right_side_dir"));
}

/// With both panels on screen `Ctrl+U` swaps as it always has.
#[test]
fn ctrl_u_swaps_when_both_panels_are_shown() {
    let root = tempfile::tempdir().expect("temp dir");
    let (left_dir, right_dir) = two_directories(root.path());
    let mut app = app();
    app.left_panel
        .navigate_to(left_dir.clone())
        .expect("left panel");
    app.right_panel
        .navigate_to(right_dir.clone())
        .expect("right panel");
    swap_panels(&mut app);
    assert_eq!(app.left_panel.path, right_dir);
    assert_eq!(app.right_panel.path, left_dir);
    assert_eq!(app.status.to_string(), "Panels swapped");
}
