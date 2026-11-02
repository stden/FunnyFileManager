//! Unit tests: one panel drawn with various, empty and marked entries, a dot
//! directory's colour, and the key bar's twelve labels.

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::*;
use crate::models::DirEntry;
use crate::models::EntryKind;
use crate::models::FileName;
use crate::models::FileSize;
use crate::models::Modified;

#[test]
fn test_render_panel_with_various_entries() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();

    // Add various types of entries
    app.left_panel.entries.clear();
    app.left_panel.entries.push(DirEntry {
        name: "..".to_owned(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    });
    app.left_panel.entries.push(DirEntry {
        name: "documents".to_owned(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    });
    app.left_panel.entries.push(DirEntry {
        name: "image.jpg".to_owned(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1_536_000), // ~1.5MB,
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    });

    terminal
        .draw(|f| {
            ui(f, &mut app);
        })
        .unwrap();
}

#[test]
fn test_render_panel_with_empty_entries() {
    let mut app = App::new().unwrap();
    app.left_panel.entries.clear();
    app.left_panel.select(None);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            render_panel(f, &mut app.left_panel, area, true, None);
        })
        .unwrap();
}

#[test]
fn test_render_key_bar_variations() {
    let backend = TestBackend::new(120, 3);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            render_key_bar(f, area, &FKEY_LABELS);
        })
        .unwrap();

    let small_backend = TestBackend::new(40, 3);
    let mut small_terminal = Terminal::new(small_backend).unwrap();

    small_terminal
        .draw(|f| {
            let area = f.area();
            render_key_bar(f, area, &FKEY_LABELS);
        })
        .unwrap();
}

#[test]
fn test_render_panel_with_marked_files() {
    let mut app = App::new().unwrap();

    for i in 0..10 {
        app.left_panel.entries.push(DirEntry {
            name: format!("file{i}.txt"),
            kind: EntryKind::File,
            size: FileSize::from_bytes(100 * i),
            modified: Modified::UNKNOWN,
            executable: false,
            ..DirEntry::default()
        });
    }

    app.left_panel.marked.insert(FileName::from("file2.txt"));
    app.left_panel.marked.insert(FileName::from("file5.txt"));
    app.left_panel.marked.insert(FileName::from("file8.txt"));
    app.left_panel.select(Some(5));

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            render_panel(f, &mut app.left_panel, area, true, None);
        })
        .unwrap();
}

/// The screen column where `text` starts on row `y`, or None.
#[cfg(not(windows))]
fn column_of(buf: &ratatui::buffer::Buffer, y: u16, text: &str) -> Option<u16> {
    let row: String = (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_owned())
        .collect();
    row.split_once(text)
        .map(|(before, _)| u16::try_from(before.chars().count()).expect("column fits u16"))
}

/// A leading dot is a Unix naming convention, not a hidden attribute, so a
/// `.dotdir` is painted like any other directory: white, as Far paints it.
/// Only Windows has a hidden bit to dim for.
#[cfg(not(windows))]
#[test]
fn test_a_dot_directory_is_painted_as_a_directory_not_dimmed() {
    let temp = tempfile::TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join(".dotdir")).expect("mkdir");
    let mut app = App::new().expect("test app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let buf = terminal.backend().buffer();

    let (x, y) = (0..24)
        .find_map(|y| column_of(buf, y, ".dotdir").map(|x| (x, y)))
        .expect("the entry is on screen");
    assert_eq!(buf[(x, y)].fg, palette::DIRECTORY);
}

/// The panels' bar is Far's twelve labels, cell for cell and colour for
/// colour, at the classic width. Pinned so the generic key bar provably draws
/// the panels exactly as the panel-only one did.
#[test]
fn the_panel_key_bar_is_fars_twelve_labels_at_eighty_columns() {
    let mut terminal = Terminal::new(TestBackend::new(80, 1)).unwrap();
    terminal
        .draw(|f| render_key_bar(f, f.area(), &FKEY_LABELS))
        .unwrap();
    let buf = terminal.backend().buffer();
    let row: String = (0..80).map(|x| buf[(x, 0)].symbol()).collect();
    assert_eq!(
        row,
        "1Help  2UserMn3View  4Edit  5Copy  6RenMov7MkFold8Delete9Conf…10Quit11Plu…12Scr…"
    );
    assert_eq!(buf[(0, 0)].bg, Color::Black);
    assert_eq!(buf[(0, 0)].fg, palette::DIRECTORY);
    assert_eq!(buf[(1, 0)].bg, palette::CURSOR_BG);
    assert_eq!(buf[(1, 0)].fg, palette::CURSOR_FG);
}
