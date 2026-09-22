//! Unit tests: every dialog the completed F9 menu opens draws its frame, and
//! the panel area follows `panels_shown`, the view mode and the panel kind.

use ratatui::Terminal;

use super::{render_overlays, ui};
use crate::menu::SettingsPage as OptionsPage;
use crate::models::{
    App, ApplyPrompt, ArchiveMenu, DescribePrompt, FilterPrompt, HistoryList, Overlay, PanelKind,
    PanelSide, PanelView, PanelsShown, ScreenList, ShortcutList, SortMenu,
};
use crate::test_support::{buffer_text, WritableTestBackend};

fn terminal() -> Terminal<WritableTestBackend> {
    Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal")
}

fn every_menu_dialog() -> Vec<Overlay> {
    vec![
        Overlay::ArchivePrompt(Box::default()),
        Overlay::ExtractPrompt(Box::default()),
        Overlay::ArchiveCommands(ArchiveMenu::default()),
        Overlay::WipeConfirm(Vec::new()),
        Overlay::ApplyCommand(ApplyPrompt::default()),
        Overlay::Attributes(Box::default()),
        Overlay::SortModes(SortMenu::default()),
        Overlay::Describe(DescribePrompt::default()),
        Overlay::FindFile(Box::default()),
        Overlay::CommandHistory(HistoryList::default()),
        Overlay::FindFolder(Box::default()),
        Overlay::ViewHistory(HistoryList::default()),
        Overlay::FolderHistory(HistoryList::default()),
        Overlay::UserMenu(Box::default()),
        Overlay::Associations(Box::default()),
        Overlay::FolderShortcuts(ShortcutList::default()),
        Overlay::Filter(FilterPrompt::default()),
        Overlay::Screens(ScreenList::default()),
        Overlay::Tasks(Box::default()),
        Overlay::Hotplug(Box::default()),
        Overlay::Settings(Box::new(crate::models::SettingsDialog::for_page(
            OptionsPage::Colors,
        ))),
    ]
}

/// Every completed F9 dialog paints a frame and records it so a click on its
/// button lands. Placeholder dialogs still say they are coming.
#[test]
fn every_menu_dialog_draws_its_frame() {
    for overlay in every_menu_dialog() {
        let mut terminal = terminal();
        let mut app = App::new().expect("test app");
        app.set_overlay(overlay);
        terminal
            .draw(|f| render_overlays(f, &mut app))
            .expect("draw");
        let screen = buffer_text(&terminal);
        assert!(!screen.trim().is_empty(), "{screen}");
        assert!(app.dialog_mouse.area.is_some(), "the frame was tracked");
    }
}

#[test]
fn the_settings_dialog_is_titled_after_its_page() {
    let mut terminal = terminal();
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::Settings(Box::new(
        crate::models::SettingsDialog::for_page(OptionsPage::Viewer),
    )));
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    let screen = buffer_text(&terminal);
    assert!(screen.contains("Viewer settings"), "{screen}");
}

fn draw(app: &mut App) -> String {
    let mut terminal = terminal();
    terminal.draw(|f| ui(f, app)).expect("draw");
    buffer_text(&terminal)
}

#[test]
fn a_tree_and_a_quick_view_panel_draw_their_own_frames() {
    let mut app = App::new().expect("test app");
    app.left_panel.kind = PanelKind::Tree(Box::default());
    app.right_panel.kind = PanelKind::QuickView(Box::default());
    let screen = draw(&mut app);
    assert!(screen.contains(" Tree "), "{screen}");
    assert!(screen.contains("Quick view"), "{screen}");
}

/// With one panel hidden the other has the whole width, so its frame's right
/// edge is the screen's.
#[test]
fn a_hidden_panel_gives_its_half_to_the_other() {
    let mut app = App::new().expect("test app");
    app.panels_shown = PanelsShown::LeftOnly;
    app.left_panel.kind = PanelKind::Tree(Box::default());
    app.right_panel.kind = PanelKind::QuickView(Box::default());
    let screen = draw(&mut app);
    assert!(screen.contains(" Tree "), "{screen}");
    assert!(
        !screen.contains("Quick view"),
        "the right panel is off screen"
    );
    let second_row = screen.lines().nth(1).expect("a panel row");
    assert!(
        second_row.trim_end().len() >= 79,
        "the left frame spans the width: {second_row:?}"
    );
}

#[test]
fn a_fullscreen_view_on_the_active_panel_takes_the_whole_area() {
    let mut app = App::new().expect("test app");
    app.active_panel = PanelSide::Right;
    app.right_panel.view = PanelView::Detailed;
    app.left_panel.kind = PanelKind::Tree(Box::default());
    let screen = draw(&mut app);
    assert!(
        !screen.contains("Folder tree"),
        "the left panel is off screen: {screen}"
    );
}
