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
fn read_uses_the_first_existing_file_and_tolerates_none() -> io::Result<()> {
    let temp = TempDir::new()?;
    let names = default_names();
    assert_eq!(read(temp.path(), &names).len(), 0);
    fs::write(temp.path().join("Files.bbs"), "a.txt from bbs\n")?;
    assert_eq!(read(temp.path(), &names)["a.txt"], "from bbs");
    fs::write(temp.path().join("Descript.ion"), "a.txt from ion\n")?;
    assert_eq!(read(temp.path(), &names)["a.txt"], "from ion");
    Ok(())
}

#[test]
fn set_description_rewrites_only_the_named_line() -> io::Result<()> {
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
fn set_description_creates_the_first_name_when_none_exists() -> io::Result<()> {
    let temp = TempDir::new()?;
    set_description(temp.path(), &default_names(), "a.txt", "Fresh")?;
    assert_eq!(read(temp.path(), &default_names())["a.txt"], "Fresh");
    assert!(temp.path().join("Descript.ion").is_file());
    let error = set_description(temp.path(), &[], "a.txt", "x").expect_err("no names");
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    Ok(())
}

#[cfg(unix)]
#[test]
fn set_description_refuses_dangling_symlink_instead_of_writing_target() -> io::Result<()> {
    let temp = TempDir::new()?;
    let outside = TempDir::new()?;
    let victim = outside.path().join("victim.txt");
    std::os::unix::fs::symlink(&victim, temp.path().join("Descript.ion"))?;

    let err = set_description(temp.path(), &default_names(), "a.txt", "note")
        .expect_err("dangling symlink must be refused");
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    assert!(
        !victim.exists(),
        "must not create file at dangling symlink target"
    );
    Ok(())
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("listing")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// Far treats `descript.ion`, `Descript.ion` and `DESCRIPT.ION` as one file.
/// Reading finds it whatever its case, and Ctrl+Z writes back to that file
/// instead of creating a second one beside it.
#[test]
fn the_description_file_is_found_and_rewritten_whatever_its_case() -> io::Result<()> {
    for spelling in ["descript.ion", "DESCRIPT.ION", "Descript.ion"] {
        let temp = TempDir::new()?;
        fs::write(temp.path().join(spelling), "a.txt Old\nb.txt Kept\n")?;
        assert_eq!(
            read(temp.path(), &default_names())["a.txt"],
            "Old",
            "{spelling}"
        );
        set_description(temp.path(), &default_names(), "a.txt", "New")?;
        assert_eq!(names_in(temp.path()), [spelling], "one description file");
        assert_eq!(
            fs::read_to_string(temp.path().join(spelling))?,
            "b.txt Kept\na.txt New\n"
        );
    }
    Ok(())
}

/// The configured order still decides between two different files, whatever
/// case each is spelled in.
#[test]
fn the_search_order_holds_across_letter_case() -> io::Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join("files.bbs"), "a.txt from bbs\n")?;
    fs::write(temp.path().join("DESCRIPT.ION"), "a.txt from ion\n")?;
    assert_eq!(read(temp.path(), &default_names())["a.txt"], "from ion");
    Ok(())
}

/// Among one listing's names the first configured name wins, and within one
/// name the exact spelling beats another spelling, whatever the listing order.
#[test]
fn choose_takes_the_first_configured_name_then_the_exact_spelling() {
    let names = default_names();
    let listed = |files: &[&str]| files.iter().map(OsString::from).collect::<Vec<_>>();
    assert_eq!(
        choose(&names, &listed(&["notes.txt", "descript.ion"])),
        Some(OsStr::new("descript.ion"))
    );
    assert_eq!(
        choose(&names, &listed(&["files.bbs", "DESCRIPT.ION"])),
        Some(OsStr::new("DESCRIPT.ION"))
    );
    for order in [
        ["descript.ion", "Descript.ion"],
        ["Descript.ion", "descript.ion"],
    ] {
        assert_eq!(
            choose(&names, &listed(&order)),
            Some(OsStr::new("Descript.ion")),
            "{order:?}"
        );
    }
    assert_eq!(choose(&names, &listed(&["notes.txt"])), None);
}

/// [`read_listed`] takes the description file from the names it is given, so a
/// description file the listing does not name is not read, even when it is there.
#[test]
fn read_listed_reads_only_the_names_it_is_given() -> io::Result<()> {
    let temp = TempDir::new()?;
    let names = default_names();
    fs::write(temp.path().join("descript.ion"), "a.txt From the listing\n")?;
    let listed = [OsString::from("a.txt"), OsString::from("descript.ion")];
    assert_eq!(
        read_listed(temp.path(), &names, &listed)["a.txt"],
        "From the listing"
    );
    assert!(read_listed(temp.path(), &names, &listed[..1]).is_empty());
    Ok(())
}
