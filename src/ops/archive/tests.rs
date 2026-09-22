//! Unit tests: the archive runs behind the three dialogs — the command each
//! format builds, the transcript line that keeps a strange name one word, the
//! archives `tar` really writes, and the members an extraction refuses.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

#[cfg(unix)]
use super::command_line;
use super::{create_archive, member_type, on_path, validate_listing, ArchiveRequest};
use crate::models::ArchiveFormat;

fn request<'a>(
    archive: &'a Path,
    format: ArchiveFormat,
    sources: &'a [String],
) -> ArchiveRequest<'a> {
    ArchiveRequest {
        archive,
        format,
        sources,
        append: false,
    }
}

fn names(sources: &[&str]) -> Vec<String> {
    sources.iter().map(|name| (*name).to_owned()).collect()
}

#[cfg(unix)]
#[test]
fn each_format_builds_its_own_command() {
    let sources = names(&["a.txt", "dir"]);
    let archive = Path::new("/out/x.tar.gz");
    assert_eq!(
        command_line(&request(archive, ArchiveFormat::TarGz, &sources)),
        "tar -czf /out/x.tar.gz -- a.txt dir"
    );
    assert_eq!(
        command_line(&request(
            Path::new("/out/x.tar"),
            ArchiveFormat::Tar,
            &sources
        )),
        "tar -cf /out/x.tar -- a.txt dir"
    );
    assert_eq!(
        command_line(&request(
            Path::new("/out/x.zip"),
            ArchiveFormat::Zip,
            &sources
        )),
        "zip -rq /out/x.zip a.txt dir"
    );
}

/// A name with a space, a quote or a `$` is echoed as exactly one shell word,
/// the way the transcript's reader would have to type it.
#[cfg(unix)]
#[test]
fn strange_names_stay_one_argument_each() {
    let sources = names(&["it's here.txt", "$HOME"]);
    let line = command_line(&request(
        Path::new("/o/a b.tar"),
        ArchiveFormat::Tar,
        &sources,
    ));
    assert_eq!(line, "tar -cf '/o/a b.tar' -- 'it'\\''s here.txt' '$HOME'");
}

#[cfg(unix)]
#[test]
fn tar_gz_and_tar_archives_are_really_written() {
    let dir = TempDir::new().expect("dir");
    fs::write(dir.path().join("it's here.txt"), "x").expect("write");
    fs::create_dir(dir.path().join("sub")).expect("mkdir");
    fs::write(dir.path().join("sub").join("inner.txt"), "y").expect("write");
    let sources = names(&["it's here.txt", "sub"]);

    for (format, name) in [
        (ArchiveFormat::TarGz, "out.tar.gz"),
        (ArchiveFormat::Tar, "out.tar"),
    ] {
        let archive = dir.path().join(name);
        let output =
            create_archive(&request(&archive, format, &sources), dir.path()).expect("runs");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let listed = std::process::Command::new("tar")
            .arg("tf")
            .arg(&archive)
            .output()
            .expect("tar t");
        let listing = String::from_utf8_lossy(&listed.stdout);
        assert!(listing.contains("it's here.txt"), "{listing}");
        assert!(listing.contains("sub/inner.txt"), "{listing}");
    }
}

/// Only where `zip` is installed: the run goes through the same road.
#[cfg(unix)]
#[test]
fn zip_archives_are_written_where_zip_exists() {
    if !on_path("zip") {
        return;
    }
    let dir = TempDir::new().expect("dir");
    fs::write(dir.path().join("a.txt"), "x").expect("write");
    let sources = names(&["a.txt"]);
    let archive = dir.path().join("out.zip");
    let output =
        create_archive(&request(&archive, ArchiveFormat::Zip, &sources), dir.path()).expect("runs");
    assert!(output.status.success());
    assert!(archive.is_file());
}

/// `zip` is offered only where it is: a radio button that fails when pressed
/// is worse than one that is not there.
#[test]
fn the_offered_formats_follow_the_path() {
    let offered = super::offered_formats();
    assert_eq!(&offered[..2], [ArchiveFormat::TarGz, ArchiveFormat::Tar]);
    assert_eq!(offered.contains(&ArchiveFormat::Zip), on_path("zip"));
    assert!(on_path("tar"));
    assert!(!on_path("no-such-program-ffm"));
}

/// A tool that is not there is an error, not a hang or a silent nothing.
#[test]
fn a_missing_tool_is_an_error() {
    let dir = TempDir::new().expect("dir");
    let sources = names(&["a.txt"]);
    let archive = dir.path().join("out.zip");
    if on_path("zip") {
        return;
    }
    let result = create_archive(&request(&archive, ArchiveFormat::Zip, &sources), dir.path());
    assert!(
        result.is_err() || !result.expect("output").status.success(),
        "zip is absent here"
    );
}

/// The Archive commands row adds to a plain tar with `tar -r`: the old
/// members stay and the new one joins them.
#[cfg(unix)]
#[test]
fn an_existing_tar_grows_when_appended() {
    let dir = TempDir::new().expect("dir");
    fs::write(dir.path().join("first.txt"), "1").expect("write");
    fs::write(dir.path().join("second.txt"), "2").expect("write");
    let archive = dir.path().join("out.tar");
    let first = names(&["first.txt"]);
    create_archive(&request(&archive, ArchiveFormat::Tar, &first), dir.path()).expect("runs");
    let second = names(&["second.txt"]);
    let mut appended = request(&archive, ArchiveFormat::Tar, &second);
    appended.append = true;
    assert_eq!(
        command_line(&appended),
        format!("tar -rf {} -- second.txt", archive.display())
    );
    let output = create_archive(&appended, dir.path()).expect("runs");
    assert!(output.status.success());
    let listing = super::list_archive(&archive).expect("listing").join("\n");
    assert!(
        listing.contains("first.txt") && listing.contains("second.txt"),
        "{listing}"
    );
}

/// A member that would land outside the destination, or a link, is refused
/// before any tool writes — whichever tool listed it.
#[test]
fn extraction_rejects_parent_paths_and_links() {
    assert!(validate_listing("../outside", "-rw-r--r-- file").is_err());
    assert!(validate_listing("/outside", "-rw-r--r-- file").is_err());
    assert!(validate_listing("C:/outside", "-rw-r--r-- file").is_err());
    assert!(validate_listing("link", "lrwxrwxrwx link -> /outside").is_err());
    assert!(validate_listing("directory/file", "-rw-r--r-- directory/file").is_ok());
    let zip_details = "Archive:  a.zip\nZip file size: 363 bytes, number of entries: 2\n\
        -rw-r--r--  2.0 unx        3 b- stor 26-Sep-17 17:16 hello.txt\n\
        ?rw-------  2.0 unx        1 b- stor 26-Sep-17 17:16 plain.txt\n\
        2 files, 4 bytes uncompressed, 4 bytes compressed:  0.0%";
    assert!(validate_listing("hello.txt\nplain.txt", zip_details).is_ok());
    let with_link =
        format!("{zip_details}\nlrwxrwxrwx  2.0 unx       11 b- stor 80-Jan-01 00:00 link");
    assert!(validate_listing("hello.txt\nplain.txt\nlink", &with_link).is_err());
}

/// Only a line shaped like a mode is a member; `unzip -Z`'s header and
/// footer are not.
#[test]
fn a_member_line_is_told_by_its_mode() {
    assert_eq!(
        member_type("drwxr-xr-x root/root 0 2026-09-17 payload/"),
        Some('d')
    );
    assert_eq!(member_type("lrwxrwxrwx"), Some('l'));
    assert_eq!(member_type("Archive:  a.zip"), None);
    assert_eq!(member_type("3 files, 15 bytes uncompressed"), None);
    assert_eq!(member_type("short"), None);
}

/// A plain file is refused by name, with the accepted kinds spelled out.
#[test]
fn a_plain_file_is_not_an_archive_to_list_or_test() {
    let dir = TempDir::new().expect("dir");
    let plain = dir.path().join("readme.md");
    fs::write(&plain, "# hi\n").expect("write");
    let error = super::list_archive(&plain).expect_err("not an archive");
    assert!(
        error.to_string().contains(".zip"),
        "names what is accepted: {error}"
    );
    assert!(super::test_archive(&plain).is_err());
}
