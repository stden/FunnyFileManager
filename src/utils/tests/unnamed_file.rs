//! Purpose: unit tests for a file that has no name until it is whole.

use super::*;

use tempfile::TempDir;

/// Nothing is in the folder while the file is written, so a process killed
/// then leaves nothing; once named, it is an ordinary file there.
#[cfg(target_os = "linux")]
#[test]
fn an_unnamed_file_appears_only_once_it_is_named() -> io::Result<()> {
    let dir = TempDir::new()?;
    let source = TempDir::new()?;
    let original = source.path().join("original");
    fs::write(&original, b"whole")?;
    let Some(mut file) = unnamed_file_in(dir.path()) else {
        eprintln!("skipped: this filesystem has no O_TMPFILE");
        return Ok(());
    };
    write_contents(&open_regular_file(&original)?, &mut file, CopyRules::PLAIN)?;
    assert_eq!(fs::read_dir(dir.path())?.count(), 0, "nothing named yet");

    let path = dir.path().join("named");
    name_unnamed(&file, &path)?;

    assert_eq!(fs::read(&path)?, b"whole");
    let taken = name_unnamed(&file, &path).expect_err("the name is taken");
    assert_eq!(taken.kind(), io::ErrorKind::AlreadyExists);
    Ok(())
}

#[test]
fn a_folder_that_is_not_there_makes_no_unnamed_file() {
    let dir = TempDir::new().expect("test setup");
    assert!(unnamed_file_in(&dir.path().join("missing")).is_none());
}
