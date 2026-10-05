use super::*;
use crate::{
    menu::{Heading, Menu, SettingsPage},
    models::{App, FileName, PanelView, PanelsShown},
};
use ratatui::{backend::TestBackend, Terminal};

pub(super) fn fixture() -> (tempfile::TempDir, App) {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join("folder")).unwrap();
    std::fs::write(temp.path().join("longfilename.extension"), "content").unwrap();
    let panel = Panel::new(temp.path().to_owned()).unwrap();
    (temp, App::from_panels(panel.clone(), panel))
}

pub(super) fn screen(app: &mut App, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| crate::ui::ui(f, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

pub(super) fn row_text(panel: &Panel, entry: &DirEntry, name: &str, width: u16) -> String {
    columns::Layout::new(panel.view, Cells::columns(width)).row(panel, entry, name)
}

#[test]
fn every_panel_mode_renders_its_cached_details_at_small_and_large_sizes() {
    let (_temp, mut app) = fixture();
    let name = "longfilename.extension".to_owned();
    app.left_panel
        .details
        .descriptions
        .insert(name.clone(), "File description".to_owned());
    app.left_panel
        .details
        .owners
        .insert(name.clone(), "test-owner".to_owned());
    app.left_panel
        .details
        .links
        .insert(name.clone(), "2".to_owned());
    app.left_panel.marked.insert(FileName::from(name.as_str()));
    // Under the cursor, so the info row carries the whole name even where
    // Brief's narrow columns cut it short to keep the extension aligned.
    let at = app.left_panel.entries.iter().position(|e| e.name == name);
    app.left_panel.select(at);
    for mode in PanelView::ALL {
        app.left_panel.view = mode;
        app.right_panel.view = mode;
        assert!(
            screen(&mut app, 120, 30).contains("longfilename"),
            "{mode:?}"
        );
        for size in [(1, 1), (10, 4), (20, 7), (40, 10)] {
            screen(&mut app, size.0, size.1);
        }
    }
}

#[test]
fn description_owner_and_link_views_use_the_correct_cached_field() {
    let (_temp, mut app) = fixture();
    let entry = app
        .left_panel
        .entries
        .iter()
        .find(|e| e.name == "longfilename.extension")
        .unwrap()
        .clone();
    app.left_panel
        .details
        .descriptions
        .insert(entry.name.clone(), "Description".to_owned());
    app.left_panel
        .details
        .owners
        .insert(entry.name.clone(), "Owner".to_owned());
    app.left_panel
        .details
        .links
        .insert(entry.name.clone(), "3".to_owned());
    for (mode, text) in [
        (PanelView::Descriptions, "Description"),
        (PanelView::Owners, "Owner"),
        (PanelView::Links, "3"),
    ] {
        app.left_panel.view = mode;
        assert!(row_text(&app.left_panel, &entry, &entry.name, 80).contains(text));
    }
}

#[test]
fn short_names_render_in_far_s_8_3_form_without_changing_the_file_identity() {
    let (_temp, mut app) = fixture();
    app.left_panel.options.short_names = true;
    // The full layout and an alternative one draw rows through different
    // code; both must look the name up in the same table.
    for view in [PanelView::Full, PanelView::Wide] {
        app.left_panel.view = view;
        let rendered = screen(&mut app, 120, 30);
        // The full layout aligns the extension to the column's edge, so only
        // the stem is contiguous there; the wide one shows the name whole.
        assert!(rendered.contains("LONGFI~1"), "{view:?}");
        assert!(
            rendered.contains("EXT"),
            "the 8.3 extension is on the row: {view:?}"
        );
        assert!(
            rendered.contains("longfilename"),
            "the right panel still shows long names"
        );
    }
    assert!(app
        .left_panel
        .entries
        .iter()
        .any(|e| e.name == "longfilename.extension"));
    assert!(short_names(&app.right_panel).is_none());
    let table = short_names(&app.left_panel).expect("short names are on");
    assert_eq!(displayed_name(Some(&table), ".."), "..");
    assert_eq!(displayed_name(Some(&table), "folder"), "FOLDER");
    assert_eq!(displayed_name(Some(&table), "unlisted"), "unlisted");
    assert_eq!(
        displayed_name(None, "longfilename.extension"),
        "longfilename.extension"
    );
}

#[test]
fn menu_scrolling_translation_visibility_and_colors_render_from_settings() {
    let (temp, mut app) = fixture();
    app.settings.set(SettingsPage::Colors, "black").unwrap();
    app.settings.set(SettingsPage::Highlighting, "off").unwrap();
    app.settings.set(SettingsPage::Interface, "off").unwrap();
    app.settings
        .translations
        .insert("&Brief".to_owned(), "&Compact".to_owned());
    app.set_menu(Menu::open(Heading::Left));
    assert!(screen(&mut app, 100, 24).contains("Compact"));
    let mut menu = Menu::open(Heading::Options);
    menu.row = menu.entries().len() - 1;
    app.set_menu(menu);
    assert!(screen(&mut app, 80, 12).contains("Save setup"));
    app.close_overlay();
    app.right_panel = Panel::new(temp.path().join("folder")).unwrap();
    app.panels_shown = PanelsShown::RightOnly;
    assert!(!screen(&mut app, 120, 30).contains("longfilename"));
    app.settings.set(SettingsPage::Colors, "gray").unwrap();
    screen(&mut app, 20, 8);
}
