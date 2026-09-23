//! Unit tests: extraction never replaces what the destination already holds,
//! and never writes through a link it finds there — whichever tool unpacks.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::extract_archive;
use super::tests_extract::{has, pack_files};

/// Every format the tests can build on this host.
fn formats() -> Vec<&'static str> {
    let mut formats = vec!["a.tar", "a.tar.gz"];
    if has("unzip") && has("python3") {
        formats.push("a.zip");
    }
    formats
}

/// Two members, one of which the destination already has, edited.
const MEMBERS: &[(&str, &str)] = &[
    ("docs/README.md", "# Readme"),
    ("docs/medium.txt", "forty lines"),
];

/// The user's edited file survives, and the archive's other member does not
/// arrive either: a refused extraction writes nothing.
#[test]
fn an_existing_file_is_never_replaced_and_nothing_is_extracted() {
    for format in formats() {
        let temp = TempDir::new().expect("temp");
        let archive = pack_files(temp.path(), format, MEMBERS);
        let destination = temp.path().join("dest-dir");
        fs::create_dir_all(destination.join("docs")).expect("docs");
        fs::write(destination.join("docs/README.md"), "MY LOCAL EDITS").expect("edit");

        let error = extract_archive(&archive, &destination).expect_err(format);

        let text = error.to_string();
        let clash = Path::new("docs").join("README.md");
        assert!(
            text.contains(&clash.display().to_string()),
            "{format}: {text}"
        );
        assert!(text.contains("already exists"), "{format}: {text}");
        let kept = fs::read_to_string(destination.join("docs/README.md")).expect("read");
        assert_eq!(kept, "MY LOCAL EDITS", "{format}");
        assert!(!destination.join("docs/medium.txt").exists(), "{format}");
        assert_only(&destination, &["docs"], format);
    }
}

/// A folder the destination already has is merged into, not replaced.
#[test]
fn an_existing_folder_is_merged_into() {
    for format in formats() {
        let temp = TempDir::new().expect("temp");
        let archive = pack_files(temp.path(), format, MEMBERS);
        let destination = temp.path().join("dest-dir");
        fs::create_dir_all(destination.join("docs")).expect("docs");
        fs::write(destination.join("docs/mine.txt"), "mine").expect("mine");

        extract_archive(&archive, &destination).expect(format);

        let read = |name: &str| fs::read_to_string(destination.join(name)).expect(name);
        assert_eq!(read("docs/mine.txt"), "mine", "{format}");
        assert_eq!(read("docs/README.md"), "# Readme", "{format}");
        assert_eq!(read("docs/medium.txt"), "forty lines", "{format}");
        assert_only(&destination, &["docs"], format);
    }
}

/// `dest-dir/escape -> ../target-dir` and a member `escape/pwned.txt`: the
/// member is refused, not written into `target-dir`.
#[cfg(unix)]
#[test]
fn a_link_already_in_the_destination_is_never_written_through() {
    for format in formats() {
        let temp = TempDir::new().expect("temp");
        let archive = pack_files(temp.path(), format, &[("escape/pwned.txt", "PWNED")]);
        let destination = temp.path().join("dest-dir");
        let target = temp.path().join("target-dir");
        fs::create_dir_all(&destination).expect("dest");
        fs::create_dir_all(&target).expect("target");
        std::os::unix::fs::symlink("../target-dir", destination.join("escape")).expect("link");

        let error = extract_archive(&archive, &destination).expect_err(format);

        assert!(error.to_string().contains("escape"), "{format}: {error}");
        assert!(
            !target.join("pwned.txt").exists(),
            "{format}: written outside"
        );
        assert_only(&destination, &["escape"], format);
    }
}

/// What unpacked must be plain files and folders before any of it moves
/// into the destination; a link that got past the listing stops it there.
#[cfg(unix)]
#[test]
fn a_link_unpacked_into_the_staging_folder_is_refused() {
    let temp = TempDir::new().expect("temp");
    fs::create_dir_all(temp.path().join("docs")).expect("docs");
    fs::write(temp.path().join("docs/a.txt"), "a").expect("file");
    std::os::unix::fs::symlink("/etc", temp.path().join("docs/etc")).expect("link");

    let error = super::extract::plain_tree(temp.path()).expect_err("a link");

    assert!(error.to_string().contains("docs/etc"), "{error}");
}

/// Nothing but `names` (and no staging folder) is left in `destination`.
fn assert_only(destination: &Path, names: &[&str], format: &str) {
    let mut listed: Vec<String> = fs::read_dir(destination)
        .expect("list")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    listed.sort_unstable();
    assert_eq!(listed, names, "{format}");
}
