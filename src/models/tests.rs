//! Unit tests and shared test helpers for `models`.
#![allow(unused_imports)]

use crate::models::BookmarkSlot;
use crate::models::EntryKind;
use crate::models::FileSize;
use crate::models::Modified;
use crate::models::{Bookmarks, FileName};
use crate::scroll::ScrollTop;
use std::fs;

use super::*;

use std::fs::File;

use std::io::Write;

use tempfile::TempDir;

pub fn file_names(panel: &Panel) -> Vec<String> {
    panel
        .entries
        .iter()
        .filter(|entry| !entry.kind.is_dir() && entry.name != "..")
        .map(|entry| entry.name.clone())
        .collect()
}

pub fn editor_from(content: &str) -> Editor {
    Editor::from_content("test.txt".to_string(), PathBuf::from("test.txt"), content)
}

#[cfg(windows)]
#[test]
fn test_windows_drive_paths_from_mask() {
    let drives = windows_drive_paths_from_mask((1 << 2) | (1 << 3) | (1 << 25));
    assert_eq!(
        drives,
        vec![
            PathBuf::from("C:\\"),
            PathBuf::from("D:\\"),
            PathBuf::from("Z:\\")
        ]
    );
}

#[test]
fn test_dir_entry_creation() {
    let entry = DirEntry {
        name: "test.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1024),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };

    assert_eq!(entry.name, "test.txt");
    assert!(!entry.kind.is_dir());
    assert_eq!(entry.size, FileSize::from_bytes(1024));
}

#[test]
fn test_app_swap_panels() -> io::Result<()> {
    let temp_dir1 = TempDir::new().expect("test setup");
    let temp_dir2 = TempDir::new().expect("test setup");

    let mut app = App::new().expect("test setup");
    app.left_panel.path = temp_dir1.path().to_path_buf();
    app.right_panel.path = temp_dir2.path().to_path_buf();

    app.swap_panels();

    assert_eq!(app.left_panel.path, temp_dir2.path());
    assert_eq!(app.right_panel.path, temp_dir1.path());

    Ok(())
}

#[test]
fn test_get_drives() {
    let drives = App::get_drives();
    assert!(!drives.is_empty());
}

#[test]
fn test_get_all_drives() {
    let drives = App::get_all_drives();
    assert!(!drives.is_empty());
}

#[test]
fn test_drive_entry_display() {
    let local = DriveEntry::Local(PathBuf::from("/"));
    assert_eq!(local.display(), "/");

    let s3 = DriveEntry::S3Profile("default".to_string());
    assert_eq!(s3.display(), "S3: default");
}

#[test]
fn test_dir_entry_clone() {
    let entry = DirEntry {
        name: "test.txt".to_string(),
        kind: EntryKind::File,
        size: FileSize::from_bytes(1024),
        modified: Modified::UNKNOWN,
        executable: false,
        ..DirEntry::default()
    };

    let cloned = entry.clone();
    assert_eq!(cloned.name, entry.name);
    assert_eq!(cloned.kind.is_dir(), entry.kind.is_dir());
    assert_eq!(cloned.size, entry.size);
}

#[test]
fn test_viewer_default() {
    let viewer = Viewer::default();
    assert_eq!(viewer.title(), "");
    assert!(viewer.lines.is_empty());
    assert_eq!(viewer.scroll, ScrollTop::TOP);
}

#[test]
fn test_viewer_creation() {
    let viewer = Viewer {
        scroll: ScrollTop::TOP.down(5, usize::MAX),
        ..Viewer::report("test.txt", vec!["Line 1".to_string(), "Line 2".to_string()])
    };

    assert_eq!(viewer.title(), "test.txt");
    assert_eq!(viewer.lines.len(), 2);
    assert_eq!(viewer.scroll.lines(), 5);
}

#[test]
fn test_editor_from_content_round_trip() {
    // CRLF files must survive untouched: F4 + F2 on a .bat or a CRLF-committed
    // source file used to rewrite every line ending and produce a whole-file diff.
    for content in [
        "",
        "a",
        "a\n",
        "\n",
        "a\nb",
        "a\nb\n",
        "a\r\nb\r\n",
        "a\r\n",
    ] {
        let editor = editor_from(content);
        assert_eq!(editor.content(), content, "round trip of {content:?}");
        assert!(!editor.state.is_modified());
    }
}

#[test]
fn test_editor_keeps_crlf_while_editing() {
    let mut editor = editor_from("alpha\r\nbeta\r\n");
    assert_eq!(editor.format.line_ending, LineEnding::Crlf);
    editor.move_end();
    editor.insert_newline();
    assert_eq!(editor.content(), "alpha\r\n\r\nbeta\r\n");
}

#[test]
fn test_editor_line_ending_follows_the_majority() {
    // A single stray CRLF must not convert an otherwise LF file.
    assert_eq!(
        editor_from("a\r\nb\nc\nd\n").format.line_ending,
        LineEnding::Lf
    );
    assert_eq!(
        editor_from("a\r\nb\r\nc\n").format.line_ending,
        LineEnding::Crlf
    );
}

#[test]
fn test_editor_empty_file_has_one_line() {
    let editor = editor_from("");
    assert_eq!(editor.lines, vec![String::new()]);
}

#[test]
fn test_editor_insert_char() {
    let mut editor = editor_from("ac\n");
    editor.move_right();
    editor.insert_char('b');
    assert_eq!(editor.content(), "abc\n");
    assert_eq!(editor.cursor_col, 2);
    assert!(editor.state.is_modified());
}

#[test]
fn test_editor_insert_newline_splits_line() {
    let mut editor = editor_from("abcd\n");
    editor.move_right();
    editor.move_right();
    editor.insert_newline();
    assert_eq!(editor.content(), "ab\ncd\n");
    assert_eq!(editor.cursor_line, 1);
    assert_eq!(editor.cursor_col, 0);
}

#[test]
fn test_editor_backspace_char_and_line_join() {
    let mut editor = editor_from("ab\ncd\n");
    editor.move_down(1);
    editor.move_right();
    editor.backspace();
    assert_eq!(editor.content(), "ab\nd\n");

    editor.backspace(); // at col 0 — join with previous line
    assert_eq!(editor.content(), "abd\n");
    assert_eq!(editor.cursor_line, 0);
    assert_eq!(editor.cursor_col, 2);

    let mut at_start = editor_from("x\n");
    at_start.backspace(); // nothing before the first char
    assert_eq!(at_start.content(), "x\n");
    assert!(!at_start.state.is_modified());
}

#[test]
fn test_editor_delete_forward_char_and_line_join() {
    let mut editor = editor_from("ab\ncd\n");
    editor.delete_forward();
    assert_eq!(editor.content(), "b\ncd\n");

    editor.move_right(); // to end of "b"
    editor.delete_forward(); // join with next line
    assert_eq!(editor.content(), "bcd\n");

    editor.move_down(1);
    editor.cursor_col = editor.lines[0].chars().count();
    editor.delete_forward(); // at end of last line — no-op
    assert_eq!(editor.content(), "bcd\n");
}

#[test]
fn test_editor_unicode_editing() {
    let mut editor = editor_from("файл\n");
    editor.move_right();
    editor.move_right();
    editor.insert_char('ё');
    assert_eq!(editor.content(), "фаёйл\n");
    editor.backspace();
    assert_eq!(editor.content(), "файл\n");
    editor.delete_forward();
    assert_eq!(editor.content(), "фал\n");
}

#[test]
fn test_editor_cursor_movement_and_clamping() {
    let mut editor = editor_from("long line\nab\n");
    editor.cursor_col = 9;
    editor.move_down(1);
    assert_eq!(editor.cursor_col, 2, "column clamps to shorter line");
    editor.move_down(100);
    assert_eq!(editor.cursor_line, 1, "clamped to last line");
    editor.move_up(100);
    assert_eq!(editor.cursor_line, 0);

    // Left from col 0 wraps to end of previous line; right from EOL wraps down.
    editor.move_down(1);
    editor.cursor_col = 0;
    editor.move_left();
    assert_eq!((editor.cursor_line, editor.cursor_col), (0, 9));
    editor.move_right();
    assert_eq!((editor.cursor_line, editor.cursor_col), (1, 0));
}

#[test]
fn test_editor_save_writes_file_and_clears_modified() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let path = temp_dir.path().join("save.txt");
    fs::write(&path, "hello\n").expect("test setup");

    let mut editor = Editor::from_content("save.txt".to_string(), path.clone(), "hello\n");
    editor.cursor_col = 5;
    editor.insert_char('!');
    assert!(editor.state.is_modified());

    editor.save()?;
    assert!(!editor.state.is_modified());
    assert_eq!(fs::read_to_string(&path)?, "hello!\n");
    Ok(())
}

#[test]
fn test_editor_save_leaves_no_temp_file_behind() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let path = temp_dir.path().join("atomic.txt");
    fs::write(&path, "hello\n").expect("test setup");

    let mut editor = Editor::from_content("atomic.txt".to_string(), path.clone(), "hello\n");
    editor.move_end();
    editor.insert_char('!');
    editor.save()?;

    assert_eq!(fs::read_to_string(&path)?, "hello!\n");
    let names: Vec<String> = fs::read_dir(temp_dir.path())?
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(names, vec!["atomic.txt".to_string()]);
    Ok(())
}

#[cfg(unix)]
#[test]
fn test_editor_save_replaces_file_and_keeps_mode() -> io::Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let temp_dir = TempDir::new().expect("test setup");
    let path = temp_dir.path().join("mode.txt");
    fs::write(&path, "hello\n").expect("test setup");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("test setup");

    let mut editor = Editor::from_content("mode.txt".to_string(), path.clone(), "hello\n");
    editor.move_end();
    editor.insert_char('!');
    let inode_before = fs::metadata(&path)?.ino();
    editor.save()?;

    let after = fs::metadata(&path)?;
    // A rename swaps the file; an in-place truncate would keep the same inode
    // and would have destroyed the original before writing a single byte.
    assert_ne!(inode_before, after.ino());
    assert_eq!(after.permissions().mode() & 0o777, 0o640);
    Ok(())
}

#[test]
fn test_app_get_editor_helpers() -> io::Result<()> {
    let mut app = App::new().expect("test setup");
    assert!(app.get_editor().is_none());
    assert!(app.get_editor_mut().is_none());

    app.set_overlay(Overlay::Editor(editor_from("x\n")));
    assert!(app.get_editor().is_some());
    app.get_editor_mut().expect("editor").insert_char('y');
    assert_eq!(app.get_editor().expect("editor").content(), "yx\n");
    Ok(())
}

#[test]
fn test_panel_new() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    assert_eq!(panel.path, temp_dir.path());
    assert!(!panel.entries.is_empty());
    assert_eq!(panel.selected(), Some(0));

    panel.entries.clear();
    panel.select(None);
    assert!(panel.entries.is_empty());
    assert_eq!(panel.selected(), None);

    Ok(())
}

#[test]
fn test_panel_load_entries_empty_dir() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    panel.load_entries().expect("test setup");

    assert!(panel.entries.iter().any(|e| e.name == ".."));

    Ok(())
}

#[test]
fn test_select_after_reload_empty_entries_with_selected_name() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    panel.entries.clear();
    panel.select(Some(0));
    panel.select_after_reload(&Panel::cursor_for_test(Some("missing.txt"), 1));

    assert_eq!(panel.selected(), None);
    Ok(())
}

#[test]
fn test_select_after_reload_empty_entries_without_selected_name() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    panel.entries.clear();
    panel.select(Some(0));
    panel.select_after_reload(&Panel::cursor_for_test(None, 0));

    assert_eq!(panel.selected(), None);
    Ok(())
}

#[test]
fn test_select_after_reload_preserves_selected_name() {
    let mut panel = Panel::from_entries(
        PathBuf::new(),
        vec![
            DirEntry {
                name: "alpha.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(10),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
            DirEntry {
                name: "beta.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(20),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
        ],
    );

    panel.select(Some(0));
    panel.select_after_reload(&Panel::cursor_for_test(Some("beta.txt"), 0));

    assert_eq!(panel.selected(), Some(1));
}

#[test]
fn test_select_after_reload_stays_put_when_the_entry_is_gone() {
    let mut panel = Panel::from_entries(
        PathBuf::new(),
        vec![
            DirEntry {
                name: "alpha.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(10),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
            DirEntry {
                name: "beta.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(20),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
        ],
    );

    panel.select(Some(1));
    panel.select_after_reload(&Panel::cursor_for_test(Some("missing.txt"), 1));

    // Far leaves the cursor on the row the deleted entry occupied, so the
    // next file is under it — not back at the top of the listing.
    assert_eq!(panel.selected(), Some(1));
}

#[test]
fn test_select_after_reload_keeps_selection_when_none() {
    let mut panel = Panel::from_entries(
        PathBuf::new(),
        vec![
            DirEntry {
                name: "alpha.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(10),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
            DirEntry {
                name: "beta.txt".to_string(),
                kind: EntryKind::File,
                size: FileSize::from_bytes(20),
                modified: Modified::UNKNOWN,
                executable: false,
                ..DirEntry::default()
            },
        ],
    );

    panel.select(Some(1));
    panel.select_after_reload(&Panel::cursor_for_test(None, 0));

    assert_eq!(panel.selected(), Some(1));
}

#[test]
fn test_panel_load_entries_preserves_selection() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("a.txt"))
        .expect("test setup")
        .write_all(b"a")
        .expect("test setup");
    File::create(temp_dir.path().join("b.txt"))
        .expect("test setup")
        .write_all(b"b")
        .expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    let selected_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "b.txt")
        .expect("b.txt entry");
    panel.select(Some(selected_index));

    File::create(temp_dir.path().join("aa.txt"))
        .expect("test setup")
        .write_all(b"aa")
        .expect("test setup");
    panel.load_entries().expect("test setup");

    let selected = panel.get_selected_entry().expect("selected entry");
    assert_eq!(selected.name, "b.txt");

    Ok(())
}

#[test]
fn test_panel_load_entries_stays_put_when_selected_missing() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    File::create(temp_dir.path().join("a.txt"))
        .expect("test setup")
        .write_all(b"a")
        .expect("test setup");
    File::create(temp_dir.path().join("b.txt"))
        .expect("test setup")
        .write_all(b"b")
        .expect("test setup");

    let mut panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");
    let selected_index = panel
        .entries
        .iter()
        .position(|entry| entry.name == "b.txt")
        .expect("b.txt entry");
    panel.select(Some(selected_index));

    fs::remove_file(temp_dir.path().join("b.txt")).expect("test setup");
    panel.load_entries().expect("test setup");

    // Far leaves the cursor on the row the deleted entry had, so whatever
    // moved up into it is under the cursor — not the top of the listing.
    assert_eq!(
        panel.selected(),
        Some(selected_index.min(panel.entries.len() - 1))
    );

    Ok(())
}

#[test]
fn test_uncovered_models_methods() -> io::Result<()> {
    assert!(BookmarkSlot::new(10).is_err());
    assert!(BookmarkSlot::from_usize(11).is_err());

    let slot = BookmarkSlot::new(5).unwrap();
    assert_eq!(slot.as_u8(), 5);
    assert_eq!(slot.as_usize(), 5);

    let mut app = App::new()?;

    let overlay = app.get_overlay_mut();
    assert!(matches!(overlay, Overlay::None));

    assert!(app.get_viewer_mut().is_none());
    assert!(app.get_command_line_mut().is_none());
    assert!(app.get_new_file_input_mut().is_none());

    app.set_overlay(Overlay::Viewer(Viewer::default()));
    assert!(app.get_viewer_mut().is_some());

    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    assert!(app.get_command_line_mut().is_some());

    app.set_overlay(Overlay::NewFileInput(String::new()));
    assert!(app.get_new_file_input_mut().is_some());

    Ok(())
}

#[test]
fn test_command_line_overlay_exposes_its_buffer() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("cd /tmp".into()));

    // The accessors keep their old shape so existing call sites are unaffected.
    assert_eq!(app.get_command_line(), Some("cd /tmp"));

    app.get_command_line_mut()
        .expect("command line")
        .insert_char('/');
    assert_eq!(app.get_command_line(), Some("cd /tmp/"));
}

// ── FileName ──────────────────────────────────────────────────────────────

#[test]
fn a_file_name_looks_up_by_str_without_allocating() {
    // `Borrow<str>` is what keeps mark lookups free: the panel holds owned
    // names but every query site has a `&str` from the directory listing.
    let mut marks: std::collections::BTreeSet<FileName> = std::collections::BTreeSet::new();
    marks.insert(FileName::from("readme.md"));
    assert!(marks.contains("readme.md"));
    assert!(!marks.contains("other.md"));
    assert!(marks.remove("readme.md"));
    assert!(marks.is_empty());
}

#[test]
fn a_file_name_orders_like_the_string_it_wraps() {
    // The derived Ord must agree with `str`'s, or `Borrow<str>` lookups would
    // search a differently-sorted tree and miss.
    let mut names = [
        FileName::from("beta"),
        FileName::from("Alpha"),
        FileName::from("alpha"),
    ];
    names.sort();
    let sorted: Vec<&str> = names.iter().map(FileName::as_str).collect();
    let mut expected = vec!["beta", "Alpha", "alpha"];
    expected.sort_unstable();
    assert_eq!(sorted, expected);
}

#[test]
fn a_file_name_joins_onto_a_directory_as_one_component() {
    // `AsRef<Path>` is the one correct way to turn a mark into a path, and a
    // name is always exactly one component.
    let joined = Path::new("/srv/work").join(FileName::from("notes.txt"));
    assert_eq!(joined, Path::new("/srv/work/notes.txt"));
    assert_eq!(FileName::from("notes.txt").to_string(), "notes.txt");
}

// ── BookmarkSlot / Bookmarks ──────────────────────────────────────────────

#[test]
fn a_bookmark_slot_covers_exactly_zero_through_nine() {
    assert!(BookmarkSlot::new(0).is_ok());
    assert!(BookmarkSlot::new(BookmarkSlot::MAX).is_ok());
    assert!(BookmarkSlot::new(BookmarkSlot::MAX + 1).is_err());
    assert!(BookmarkSlot::from_usize(BookmarkSlot::COUNT).is_err());
    assert_eq!(BookmarkSlot::COUNT, usize::from(BookmarkSlot::MAX) + 1);
}

#[test]
fn every_slot_has_somewhere_to_go() {
    // The store has one entry per slot, which is what makes the lookup total
    // and let the two runtime bounds checks be deleted.
    let mut bookmarks = Bookmarks::default();
    assert_eq!(bookmarks.slots().len(), BookmarkSlot::COUNT);

    for n in 0..=BookmarkSlot::MAX {
        let slot = BookmarkSlot::new(n).expect("legal slot");
        assert_eq!(bookmarks.get(slot), None);
        bookmarks.set(slot, PathBuf::from(format!("/slot{n}")));
        assert_eq!(bookmarks.get(slot), Some(Path::new(&format!("/slot{n}"))));
        bookmarks.clear(slot);
        assert_eq!(bookmarks.get(slot), None);
    }
}

// ── EntryKind / FileSize ──────────────────────────────────────────────────

#[test]
fn a_symlink_to_a_directory_behaves_as_a_directory_and_stays_a_link() {
    // The bug the two bools caused: a symlinked directory rendered as a tiny
    // file that could not be entered, but whose whole target tree got copied.
    let kind = EntryKind::SymlinkToDirectory;
    assert!(kind.is_dir(), "Enter must descend into it");
    assert!(kind.is_symlink(), "and it is still a link");
}

#[test]
fn a_broken_link_is_neither_a_directory_nor_a_plain_file() {
    let kind = EntryKind::BrokenSymlink;
    assert!(!kind.is_dir(), "there is nothing to enter");
    assert!(kind.is_symlink());
    // The old encoding could not tell this from a symlink to a file: both were
    // (is_dir = false, is_symlink = true).
    assert_ne!(kind, EntryKind::SymlinkToFile);
}

#[test]
fn plain_entries_carry_no_link() {
    assert!(EntryKind::Directory.is_dir());
    assert!(!EntryKind::Directory.is_symlink());
    assert!(!EntryKind::File.is_dir());
    assert!(!EntryKind::File.is_symlink());
    assert!(!EntryKind::SymlinkToFile.is_dir());
    assert_eq!(EntryKind::default(), EntryKind::File);
}

#[test]
fn a_file_size_renders_at_every_unit_boundary() {
    // The rendering moved onto the type, so it cannot be applied to a line
    // count or a byte offset by accident.
    assert_eq!(FileSize::from_bytes(0).to_string(), "0 B");
    assert_eq!(FileSize::from_bytes(1023).to_string(), "1023 B");
    assert_eq!(FileSize::from_bytes(1024).to_string(), "1.00 KB");
    assert_eq!(FileSize::from_bytes(1024 * 1024).to_string(), "1.00 MB");
    assert_eq!(
        FileSize::from_bytes(1024 * 1024 * 1024).to_string(),
        "1.00 GB"
    );
}

#[test]
fn a_file_size_converts_to_megabytes_and_back_to_bytes() {
    let size = FileSize::from_bytes(3 * 1024 * 1024);
    assert!((size.megabytes() - 3.0).abs() < f64::EPSILON);
    assert_eq!(size.bytes(), 3 * 1024 * 1024);
    assert_eq!(FileSize::default(), FileSize::from_bytes(0));
    // Ordering is the byte order, so sorting a panel by size still works.
    assert!(FileSize::from_bytes(1) < FileSize::from_bytes(2));
}
