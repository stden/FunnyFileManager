//! Unit tests: the detail views' columns — the long description's share of
//! the row, Detailed's branch columns and timestamps, Alternative full's
//! grouped bytes, entry metadata without a cache override, and narrow panels.

use super::tests::{fixture, row_text, screen};
use crate::models::PanelView;

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

/// The columns where the left panel of a 100x20 screen draws a `│` on
/// `row`.
fn column_lines(app: &mut crate::models::App, row: usize) -> Vec<usize> {
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 20)).expect("terminal");
    terminal.draw(|f| crate::ui::ui(f, app)).expect("draw");
    let buffer = terminal.backend().buffer();
    (0..50)
        .filter(|&column| buffer[(column, u16::try_from(row).expect("row"))].symbol() == "│")
        .map(usize::from)
        .collect()
}

/// Below the last entry the column lines of these views stopped short, while
/// Full and Medium draw theirs down to the divider.
#[test]
fn detail_views_draw_their_column_lines_below_the_last_entry() {
    let (_temp, mut app) = fixture();
    for view in [
        PanelView::Detailed,
        PanelView::Descriptions,
        PanelView::LongDescriptions,
        PanelView::Owners,
        PanelView::Links,
        PanelView::Alternative,
    ] {
        app.left_panel.view = view;
        let header = column_lines(&mut app, 2);
        assert_ne!(header.len(), 0, "{view:?} has columns");
        // Three entries fill rows 3 to 5; row 12 is well below them.
        assert_eq!(column_lines(&mut app, 12), header, "{view:?}");
    }
}
