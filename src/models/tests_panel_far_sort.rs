//! Unit tests: Far's six extra sort modes, the header letters and reversal.

use std::{fs, io, time::Duration};

use tempfile::TempDir;

use super::tests::file_names;
use super::{Panel, SortMode};

/// `dir/`, then `old.txt` (mtime long ago), `new.txt` (now), `mid.txt`, with
/// descriptions that order them differently from their names.
fn populated() -> io::Result<TempDir> {
    let temp = TempDir::new()?;
    fs::create_dir(temp.path().join("dir"))?;
    for (name, seconds_ago) in [("old.txt", 3600), ("new.txt", 0), ("mid.txt", 1800)] {
        let path = temp.path().join(name);
        fs::write(&path, name)?;
        let stamp = std::time::SystemTime::now() - Duration::from_secs(seconds_ago);
        fs::File::options()
            .write(true)
            .open(&path)?
            .set_modified(stamp)?;
    }
    fs::write(
        temp.path().join("Descript.ion"),
        "old.txt Zulu\nnew.txt alpha\n",
    )?;
    Ok(temp)
}

fn sorted(temp: &TempDir, mode: SortMode, reversed: bool) -> io::Result<Panel> {
    let mut panel = Panel::new(temp.path().to_path_buf())?;
    panel.sort_mode = mode;
    panel.sort_reversed = reversed;
    panel.load_entries()?;
    Ok(panel)
}

#[test]
fn modification_time_puts_the_newest_first() -> io::Result<()> {
    let temp = populated()?;
    let panel = sorted(&temp, SortMode::ModificationTime, false)?;
    let files: Vec<String> = file_names(&panel)
        .into_iter()
        .filter(|name| name != "Descript.ion")
        .collect();
    assert_eq!(files, ["new.txt", "mid.txt", "old.txt"]);
    assert_eq!(panel.entries[1].name, "dir", "directories stay on top");
    Ok(())
}

#[test]
fn descriptions_sort_by_text_with_undescribed_rows_first() -> io::Result<()> {
    let temp = populated()?;
    let panel = sorted(&temp, SortMode::Descriptions, false)?;
    assert_eq!(
        file_names(&panel),
        ["Descript.ion", "mid.txt", "new.txt", "old.txt"]
    );
    Ok(())
}

#[test]
fn reversal_turns_each_group_around_but_keeps_directories_on_top() -> io::Result<()> {
    let temp = populated()?;
    let panel = sorted(&temp, SortMode::Name, true)?;
    assert_eq!(
        file_names(&panel),
        ["old.txt", "new.txt", "mid.txt", "Descript.ion"]
    );
    assert_eq!(panel.entries[0].name, "..");
    assert_eq!(panel.entries[1].name, "dir");
    Ok(())
}

#[test]
fn unsorted_keeps_the_listing_order_within_each_group() -> io::Result<()> {
    let temp = populated()?;
    let unsorted = sorted(&temp, SortMode::Unsorted, false)?;
    let mut names = file_names(&unsorted);
    assert_eq!(unsorted.entries[1].name, "dir");
    names.sort();
    assert_eq!(names, ["Descript.ion", "mid.txt", "new.txt", "old.txt"]);
    Ok(())
}

#[test]
fn creation_access_and_owner_modes_produce_a_full_listing() -> io::Result<()> {
    let temp = populated()?;
    for mode in [
        SortMode::CreationTime,
        SortMode::AccessTime,
        SortMode::Owner,
    ] {
        let panel = sorted(&temp, mode, false)?;
        assert_eq!(file_names(&panel).len(), 4, "{mode:?}");
        assert_eq!(panel.entries[1].name, "dir", "{mode:?}");
    }
    Ok(())
}

#[test]
fn every_mode_has_a_word_a_letter_and_a_label() {
    for mode in SortMode::ALL {
        let word = SortMode::USAGE.split(' ').next_back().and_then(|words| {
            words
                .split('|')
                .find(|word| SortMode::parse(word) == Some(mode))
        });
        assert!(word.is_some(), "{mode:?} is missing from the usage line");
        assert!(!mode.label().is_empty());
        assert!(mode.letter().is_ascii_lowercase());
    }
    assert_eq!(SortMode::parse("extension"), Some(SortMode::Extension));
    assert_eq!(
        SortMode::parse("modified"),
        Some(SortMode::ModificationTime)
    );
    assert_eq!(SortMode::parse("blah"), None);
    assert_eq!(SortMode::Owner.next(), SortMode::Name);
}
