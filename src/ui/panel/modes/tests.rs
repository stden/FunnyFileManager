use super::*;
use crate::{
    menu::{Heading, Menu, SettingsPage},
    models::{App, FileName, PanelView, PanelsShown},
};
use ratatui::{backend::TestBackend, Terminal};

fn fixture() -> (tempfile::TempDir, App) {
    let temp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir(temp.path().join("folder")).unwrap();
    std::fs::write(temp.path().join("longfilename.extension"), "content").unwrap();
    let panel = Panel::new(temp.path().to_owned()).unwrap();
    (temp, App::from_panels(panel.clone(), panel))
}

fn screen(app: &mut App, width: u16, height: u16) -> String {
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

fn row_text(panel: &Panel, entry: &DirEntry, name: &str, width: u16) -> String {
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

#[test]
fn long_descriptions_allocate_more_space_to_the_description() {
    let (_temp, mut app) = fixture();
    let entry = app.left_panel.entries[1].clone();
    app.left_panel
        .details
        .descriptions
        .insert(entry.name.clone(), "Description".to_owned());
    app.left_panel.view = PanelView::Descriptions;
    let normal = row_text(&app.left_panel, &entry, &entry.name, 60);
    app.left_panel.view = PanelView::LongDescriptions;
    let long = row_text(&app.left_panel, &entry, &entry.name, 60);
    assert!(long.find("Description").unwrap() < normal.find("Description").unwrap());
}

#[test]
fn detailed_view_shows_all_the_branch_columns() {
    let (_temp, mut app) = fixture();
    app.left_panel.view = PanelView::Detailed;
    let rendered = screen(&mut app, 160, 24);
    for title in [
        "Name", "Size", "Alloc", "Modified", "Created", "Accessed", "Attr",
    ] {
        assert!(rendered.contains(title), "missing column {title}");
    }
}

#[test]
fn alternative_full_groups_bytes_and_shows_the_date() {
    let (_temp, mut app) = fixture();
    app.left_panel.view = PanelView::Alternative;
    let mut entry = app
        .left_panel
        .entries
        .iter()
        .find(|e| e.name.ends_with("extension"))
        .unwrap()
        .clone();
    entry.size = crate::models::FileSize::from_bytes(1_234_567);
    let rendered = row_text(&app.left_panel, &entry, &entry.name, 60);
    assert!(rendered.contains("1,234,567"), "{rendered}");
    assert!(rendered.contains(&entry.modified.date()), "{rendered}");
}

#[test]
fn detail_views_use_entry_metadata_when_the_panel_cache_has_no_override() {
    let (_temp, mut app) = fixture();
    let mut entry = app.left_panel.entries[1].clone();
    entry.description = Some("Branch description".to_owned());
    entry.owner = Some("branch-owner".to_owned());
    entry.links = 42;
    for (view, text) in [
        (PanelView::Descriptions, "Branch description"),
        (PanelView::Owners, "branch-owner"),
        (PanelView::Links, "42"),
    ] {
        app.left_panel.view = view;
        let rendered = row_text(&app.left_panel, &entry, &entry.name, 80);
        assert!(rendered.contains(text), "{rendered}");
    }
}

#[test]
fn detailed_rows_show_allocated_bytes_and_each_distinct_timestamp() {
    use crate::models::{FileSize, Modified};
    use std::time::{Duration, UNIX_EPOCH};
    let (_temp, mut app) = fixture();
    app.left_panel.view = PanelView::Detailed;
    let mut entry = app
        .left_panel
        .entries
        .iter()
        .find(|e| e.name.ends_with("extension"))
        .unwrap()
        .clone();
    entry.size = FileSize::from_bytes(1234);
    entry.allocated = FileSize::from_bytes(4096);
    entry.modified = Modified::new(UNIX_EPOCH + Duration::from_secs(1_700_000_000));
    entry.created = Modified::new(UNIX_EPOCH + Duration::from_secs(1_600_000_000));
    entry.accessed = Modified::new(UNIX_EPOCH + Duration::from_hours(500_000));
    let rendered = row_text(&app.left_panel, &entry, &entry.name, 140);
    let cells: Vec<_> = rendered.split('│').map(str::trim).collect();
    assert_eq!(cells[1..3], ["1234", "4096"]);
    for (cell, at) in cells[3..6]
        .iter()
        .zip([entry.modified, entry.created, entry.accessed])
    {
        assert_eq!(*cell, format!("{} {}", at.date(), at.time()));
    }
    assert_eq!(cells[6], entry.attributes.to_string().trim());
}

#[test]
fn narrow_detail_views_preserve_names_and_directory_sizes_are_words() {
    let (_temp, mut app) = fixture();
    let folder = app
        .left_panel
        .entries
        .iter()
        .find(|e| e.name == "folder")
        .unwrap()
        .clone();
    for view in [
        PanelView::Detailed,
        PanelView::Alternative,
        PanelView::Owners,
        PanelView::Links,
        PanelView::LongDescriptions,
    ] {
        app.left_panel.view = view;
        assert!(row_text(&app.left_panel, &folder, &folder.name, 100).contains("Folder"));
        let narrow = row_text(&app.left_panel, &folder, &folder.name, 8);
        assert_eq!(narrow.trim(), "folder");
    }
}
