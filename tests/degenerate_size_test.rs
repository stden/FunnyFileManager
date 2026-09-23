//! Purpose: nothing panics on a terminal too small to draw in.
//!
//! Every layout in the program subtracts borders, headers and margins from the
//! area it was given. Each of those subtractions is correct at any size a user
//! would work at, and each is a potential underflow at a size nobody checked —
//! and a terminal *is* briefly 1×1 while a tiling window manager moves it.
//!
//! So rather than auditing the arithmetic site by site, this renders every
//! overlay at every size up to 24×24 and asserts the program survives. A panic
//! in a renderer takes the terminal down with it and loses whatever the user
//! had open, which makes "it never gets that small in practice" the wrong bet.

use funny_file_manager::models::{
    App, Confirm, DirEntry, EntryKind, FileName, FileSize, Modified, NamePrompt, Overlay, Panel,
};

use funny_file_manager::ui::ui;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::path::PathBuf;

/// A panel with entries, so the row-drawing code has something to draw.
///
/// Built with [`Panel::from_entries`] rather than `App::new()`: the latter
/// lists the real working directory for both panels — a `statx`, an owner
/// lookup and a `Descript.ion` probe per file — and every one of those entries
/// is then thrown away by the two assignments below. That listing was 60 % of
/// this suite's wall-clock time, and it made the test depend on whatever
/// happened to be in the checkout.
fn populated() -> App {
    let mut app = App::with_panels(
        Panel::from_entries(PathBuf::from("/srv/x"), Vec::new()),
        Panel::from_entries(PathBuf::from("/srv/x"), Vec::new()),
    );
    let entries = vec![
        DirEntry {
            name: "..".to_string(),
            kind: EntryKind::Directory,
            size: FileSize::from_bytes(0),
            modified: Modified::UNKNOWN,
            executable: false,
            ..DirEntry::default()
        },
        DirEntry {
            name: "a-rather-long-file-name.rs".to_string(),
            kind: EntryKind::File,
            size: FileSize::from_bytes(4096),
            modified: Modified::UNKNOWN,
            executable: true,
            ..DirEntry::default()
        },
    ];
    app.left_panel.entries.clone_from(&entries);
    app.right_panel.entries = entries;
    app.left_panel.select(Some(1));
    app.right_panel.select(Some(0));
    app
}

/// Every overlay that has its own renderer.
/// Each is named, because a failure reports a size and needs to say which
/// overlay was on screen at it.
fn overlays() -> Vec<(&'static str, Overlay)> {
    vec![
        ("none", Overlay::None),
        ("help", Overlay::Help),
        ("new file", Overlay::NewFileInput("name".to_string())),
        (
            "delete",
            Overlay::DeleteConfirm(vec![PathBuf::from("/tmp/x")]),
        ),
        ("quit/yes", Overlay::QuitConfirm(Confirm::Yes)),
        ("quit/no", Overlay::QuitConfirm(Confirm::No)),
        (
            "rename",
            Overlay::NamePrompt(NamePrompt::rename(FileName::new("file.txt"))),
        ),
        (
            "make folder",
            Overlay::NamePrompt(NamePrompt::make_folder()),
        ),
        ("information", Overlay::Information(Box::default())),
    ]
}

#[test]
fn every_overlay_survives_every_small_terminal() {
    let prototype = populated();
    for (name, overlay) in overlays() {
        // One `App` per overlay, cloned per size: building it is far dearer
        // than the render being measured, and it does not vary with the size.
        let mut app = prototype.clone();
        app.overlay = overlay;
        for width in 0..=24u16 {
            for height in 0..=24u16 {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                let result = terminal.draw(|frame| ui(frame, &mut app));
                assert!(
                    result.is_ok(),
                    "{width}x{height} with the {name} overlay failed: {result:?}"
                );
            }
        }
    }
}

/// The same sizes with a directory whose statistics fill the panel, since the
/// information panel lays out rows against a height it did not choose.
#[test]
fn the_information_panel_survives_a_full_report() {
    let stats = funny_file_manager::stats::analyse(std::path::Path::new("."));
    let mut app = populated();
    app.overlay = Overlay::Information(Box::new(stats));
    for width in 0..=24u16 {
        for height in 0..=24u16 {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| ui(frame, &mut app)).unwrap();
        }
    }
}
