//! Purpose: Shift+F1: each format's command, odd names, and archives really written.

use super::*;

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
    let offered = offered_formats();
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
    let listing = list_archive(&archive).expect("listing").join("\n");
    assert!(
        listing.contains("first.txt") && listing.contains("second.txt"),
        "{listing}"
    );
}
