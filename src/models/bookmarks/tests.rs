//! Purpose: bookmark slots survive a restart, and two running instances do
//! not overwrite each other's slots.

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::test_support::EnvVarGuard;

/// `unix` as an absolute path of the platform running the test. The store
/// keeps absolute paths only, and `/srv/x` has no drive on Windows.
fn fixture(unix: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("C:{}", unix.replace('/', "\\")))
    } else {
        PathBuf::from(unix)
    }
}

fn slot(number: u8) -> BookmarkSlot {
    BookmarkSlot::new(number).expect("a slot from 0 to 9")
}

/// The reported case: `bookmark set 3`, quit, start again, `bookmark go 3`.
#[test]
fn a_bookmark_set_in_one_run_is_there_in_the_next() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _xdg = EnvVarGuard::set("XDG_DATA_HOME", temp.path());
    let mut bookmarks = Bookmarks::load();
    bookmarks.remember(slot(3), fixture("/srv/x/perms"))?;

    let restarted = Bookmarks::load();
    assert_eq!(
        restarted.get(slot(3)),
        Some(fixture("/srv/x/perms").as_path())
    );
    assert_eq!(restarted.get(slot(2)), None);
    Ok(())
}

/// Beside the histories, one line per saved slot: the digit, a space, the path.
#[test]
fn the_file_lives_beside_the_histories_one_slot_per_line() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _xdg = EnvVarGuard::set("XDG_DATA_HOME", temp.path());
    let mut bookmarks = Bookmarks::load();
    bookmarks.remember(slot(7), fixture("/home/bob/with space"))?;
    bookmarks.remember(slot(0), fixture("/srv/x"))?;

    let text = fs::read_to_string(temp.path().join("ffm/bookmarks"))?;
    let expected = format!(
        "0 {}\n7 {}\n",
        fixture("/srv/x").display(),
        fixture("/home/bob/with space").display()
    );
    assert_eq!(text, expected);
    Ok(())
}

/// A second ffm started before the first set its slot must not wipe that slot
/// when it sets one of its own.
#[test]
fn two_running_instances_keep_each_others_slots() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _xdg = EnvVarGuard::set("XDG_DATA_HOME", temp.path());
    let mut first = Bookmarks::load();
    let mut second = Bookmarks::load();
    first.remember(slot(3), fixture("/srv/x/a"))?;
    second.remember(slot(4), fixture("/srv/x/b"))?;

    let restarted = Bookmarks::load();
    assert_eq!(restarted.get(slot(3)), Some(fixture("/srv/x/a").as_path()));
    assert_eq!(restarted.get(slot(4)), Some(fixture("/srv/x/b").as_path()));
    assert_eq!(second.get(slot(3)), Some(fixture("/srv/x/a").as_path()));
    Ok(())
}

/// Lines that are not a slot and a path are skipped, not fatal: a damaged file
/// costs the damaged lines only.
#[test]
fn damaged_lines_are_skipped() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _xdg = EnvVarGuard::set("XDG_DATA_HOME", temp.path());
    fs::create_dir_all(temp.path().join("ffm"))?;
    let text = format!(
        "junk\n12 {}\n5\n1 relative\n2 {}\n",
        fixture("/too/far").display(),
        fixture("/srv/x/kept").display()
    );
    fs::write(temp.path().join("ffm/bookmarks"), text)?;

    let bookmarks = Bookmarks::load();
    let saved: Vec<(usize, &Path)> = bookmarks
        .slots()
        .iter()
        .enumerate()
        .filter_map(|(index, path)| path.as_deref().map(|path| (index, path)))
        .collect();
    assert_eq!(saved, [(2, fixture("/srv/x/kept").as_path())]);
    Ok(())
}

/// The default store is session-only, so an `App` built in a test never
/// touches the user's real bookmarks.
#[test]
fn a_session_only_store_never_writes() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _xdg = EnvVarGuard::set("XDG_DATA_HOME", temp.path());
    let mut bookmarks = Bookmarks::default();
    bookmarks.remember(slot(1), fixture("/srv/x"))?;
    assert_eq!(bookmarks.get(slot(1)), Some(fixture("/srv/x").as_path()));
    assert!(!temp.path().join("ffm").exists());
    Ok(())
}

/// A path that would be two lines of the file is kept for this run only.
#[test]
fn a_path_with_a_line_break_is_not_written() -> io::Result<()> {
    let temp = TempDir::new()?;
    let _xdg = EnvVarGuard::set("XDG_DATA_HOME", temp.path());
    let mut bookmarks = Bookmarks::load();
    bookmarks.remember(slot(1), fixture("/srv/x/two\nlines"))?;
    bookmarks.remember(slot(2), fixture("/srv/x"))?;
    let text = fs::read_to_string(temp.path().join("ffm/bookmarks"))?;
    assert_eq!(text, format!("2 {}\n", fixture("/srv/x").display()));
    Ok(())
}
