//! Rendering consumes captured facts, including when the source disappears.

use std::{collections::BTreeSet, fs, io};

use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};
use tempfile::TempDir;

use crate::models::{DirectoryListing, FileName, FileSize, PanelMetadata};

use super::*;
use crate::models::DirEntry;

fn draw(panel: &mut Panel) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(80, 10)).expect("test terminal");
    terminal
        .draw(|frame| render_panel(frame, panel, frame.area(), false, None))
        .expect("render panel");
    terminal.backend().buffer().clone()
}

fn text(buffer: &Buffer) -> String {
    buffer
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

#[test]
fn hidden_colour_and_free_space_come_from_the_supplied_snapshot() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut panel = Panel::from_entries(temp.path().join("missing"), Vec::new());
    panel.apply_listing(DirectoryListing {
        entries: vec![DirEntry {
            name: "secret.txt".to_owned(),
            ..DirEntry::default()
        }],
        parent: None,
        metadata: PanelMetadata {
            hidden_names: BTreeSet::from([FileName::from("secret.txt")]),
            dirty_repos: BTreeSet::new(),
            available_space: Some(FileSize::from_bytes(1_048_576)),
            ..PanelMetadata::default()
        },
    });

    let rendered = draw(&mut panel);
    assert_eq!(rendered[(1, 2)].symbol(), "s");
    assert_eq!(rendered[(1, 2)].fg, palette::HIDDEN);
    assert!(text(&rendered).contains("1.0 M"));
    Ok(())
}

fn dir_entry(name: &str) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind: crate::models::EntryKind::Directory,
        ..DirEntry::default()
    }
}

#[test]
fn dirty_git_repository_folder_uses_dirty_repo_colour_while_clean_stays_white() -> io::Result<()> {
    let temp = TempDir::new()?;
    let mut panel = Panel::from_entries(temp.path().join("missing"), Vec::new());
    panel.apply_listing(DirectoryListing {
        entries: vec![dir_entry("clean-repo"), dir_entry("dirty-repo")],
        parent: None,
        metadata: PanelMetadata {
            hidden_names: BTreeSet::new(),
            dirty_repos: BTreeSet::from([FileName::from("dirty-repo")]),
            available_space: None,
            ..PanelMetadata::default()
        },
    });
    for view in [
        crate::models::PanelView::Full,
        crate::models::PanelView::Detailed,
    ] {
        panel.view = view;
        let rendered = draw(&mut panel);
        assert_eq!(rendered[(1, 2)].fg, palette::DIRECTORY);
        assert_eq!(rendered[(1, 3)].fg, palette::DIRTY_REPO);
    }
    Ok(())
}

#[test]
fn a_directory_disappearing_does_not_change_the_rendered_snapshot() -> io::Result<()> {
    let temp = TempDir::new()?;
    let directory = temp.path().join("listed");
    fs::create_dir(&directory)?;
    fs::write(directory.join("notes.txt"), b"notes")?;
    let mut panel = Panel::new(directory.clone())?;
    let before = draw(&mut panel);

    fs::remove_file(directory.join("notes.txt"))?;
    fs::remove_dir(&directory)?;
    assert_eq!(draw(&mut panel), before);

    assert!(panel.load_entries().is_err());
    assert_eq!(
        draw(&mut panel),
        before,
        "a failed refresh preserves all displayed facts"
    );
    Ok(())
}

#[test]
fn refreshing_publishes_changed_file_metadata() -> io::Result<()> {
    let temp = TempDir::new()?;
    let file = temp.path().join("notes.txt");
    fs::write(&file, b"a")?;
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    let before = draw(&mut panel);

    fs::write(&file, [b'x'; 2048])?;
    assert_eq!(draw(&mut panel), before);
    panel.load_entries()?;
    let after = draw(&mut panel);
    assert_ne!(after, before);
    assert!(text(&after).contains("2048"));
    Ok(())
}

/// `Ctrl+I`: the title says the listing is filtered, the way Far's does.
#[test]
fn a_filtered_panel_carries_a_star_before_its_path() -> io::Result<()> {
    let dir = TempDir::new()?;
    let mut panel = Panel::new(dir.path().to_path_buf())?;
    assert!(
        !text(&draw(&mut panel)).contains(" *"),
        "no filter, no star"
    );

    panel.filter.hide_dotfiles = true;
    let title = format!("*{}", dir.path().display());
    assert!(
        text(&draw(&mut panel)).contains(&title),
        "the star sits before the path"
    );
    Ok(())
}

#[cfg(windows)]
#[test]
fn canonical_paths_use_ordinary_spelling_in_the_panel_title() {
    use crate::ui::tail_cells;

    let temp = TempDir::new().unwrap();
    let canonical = temp.path().canonicalize().unwrap();
    let mut panel = Panel::from_entries(canonical.clone(), Vec::new());
    let rendered = text(&draw(&mut panel));
    assert!(!rendered.contains(r"\\?\"));
    assert!(rendered.contains(&tail_cells(
        &crate::path_display::label(&canonical),
        Cells::new(78)
    )));
    assert_eq!(panel.path, canonical);
}
