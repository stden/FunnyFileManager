//! Unit tests: Far's description-file format, read and rewritten.

use super::*;
use tempfile::TempDir;

#[test]
fn parses_bare_and_quoted_names() {
    let parsed = parse("notes.txt My notes\n\"my file.txt\"   Spaced name\n\nlonely\n\"broken\r\n");
    assert_eq!(
        parsed.get("notes.txt").map(String::as_str),
        Some("My notes")
    );
    assert_eq!(
        parsed.get("my file.txt").map(String::as_str),
        Some("Spaced name")
    );
    assert_eq!(parsed.get("lonely").map(String::as_str), Some(""));
    assert_eq!(
        parsed.len(),
        3,
        "an unterminated quote is skipped: {parsed:?}"
    );
}

#[test]
fn read_uses_the_first_existing_file_and_tolerates_none() -> std::io::Result<()> {
    let temp = TempDir::new()?;
    let names = default_names();
    assert!(read(temp.path(), &names).is_empty());
    fs::write(temp.path().join("Files.bbs"), "a.txt from bbs\n")?;
    assert_eq!(read(temp.path(), &names)["a.txt"], "from bbs");
    fs::write(temp.path().join("Descript.ion"), "a.txt from ion\n")?;
    assert_eq!(read(temp.path(), &names)["a.txt"], "from ion");
    Ok(())
}

#[test]
fn set_description_rewrites_only_the_named_line() -> std::io::Result<()> {
    let temp = TempDir::new()?;
    let names = default_names();
    fs::write(
        temp.path().join("Descript.ion"),
        "keep.txt Kept\n\"two words.txt\" Old\n",
    )?;
    set_description(temp.path(), &names, "two words.txt", "New")?;
    set_description(temp.path(), &names, "added.txt", "  Added  ")?;
    let text = fs::read_to_string(temp.path().join("Descript.ion"))?;
    assert_eq!(
        text,
        "keep.txt Kept\n\"two words.txt\" New\nadded.txt Added\n"
    );
    set_description(temp.path(), &names, "keep.txt", "")?;
    let text = fs::read_to_string(temp.path().join("Descript.ion"))?;
    assert_eq!(text, "\"two words.txt\" New\nadded.txt Added\n");
    Ok(())
}

#[test]
fn set_description_creates_the_first_name_when_none_exists() -> std::io::Result<()> {
    let temp = TempDir::new()?;
    set_description(temp.path(), &default_names(), "a.txt", "Fresh")?;
    assert_eq!(read(temp.path(), &default_names())["a.txt"], "Fresh");
    assert!(temp.path().join("Descript.ion").is_file());
    let error = set_description(temp.path(), &[], "a.txt", "x").expect_err("no names");
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    Ok(())
}
