//! Purpose: What counts as an archive, and each flavour unpacking, odd member names included.

use super::*;

#[test]
fn every_far_extension_is_an_archive_and_nothing_else_is() {
    for name in [
        "a.tar",
        "a.tar.gz",
        "a.tgz",
        "a.tar.xz",
        "a.tar.bz2",
        "a.zip",
        "A.ZIP",
    ] {
        assert!(is_archive(name), "{name}");
    }
    for name in [
        "a.txt",
        "tar",
        "a.gz",
        "a.rar",
        "a.7z",
        ".zip",
        "archive.zip.txt",
    ] {
        assert!(!is_archive(name), "{name}");
    }
}

#[test]
fn extract_archive_unpacks_each_tar_flavour() {
    let temp = TempDir::new().expect("temp");
    for name in ["a.tar", "b.tar.gz", "c.tgz", "d.tar.xz", "e.tar.bz2"] {
        let archive = pack(temp.path(), name);
        let destination = temp.path().join(format!("out-{name}"));
        extract_archive(&archive, &destination).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            fs::read_to_string(destination.join("payload/nested/hello.txt")).expect("unpacked"),
            "hello",
            "{name}"
        );
    }
}

#[test]
fn extract_archive_unpacks_a_zip_with_unzip() {
    if !has("unzip") || !has("python3") {
        return;
    }
    let temp = TempDir::new().expect("temp");
    let archive = pack(temp.path(), "a.zip");
    let destination = temp.path().join("out");
    extract_archive(&archive, &destination).expect("unzip");
    assert!(destination.join("payload/nested/hello.txt").is_file());
}

/// Names that are one plain relative file on Unix extract: a colon, a
/// backslash, a tab, a newline, a non-ASCII letter. `tar -tf` prints all but
/// the colon with backslash escapes, which are not a way out.
#[cfg(unix)]
#[test]
fn extract_archive_accepts_names_with_colons_backslashes_and_escapes() {
    let temp = TempDir::new().expect("temp");
    let payload = temp.path().join("names");
    fs::create_dir_all(&payload).expect("payload");
    let names = [
        "meeting 10:30.txt",
        "back\\slash",
        "tab\there",
        "newline\nhere",
        "caf\u{e9}.txt",
    ];
    for name in names {
        fs::write(payload.join(name), name).expect("file");
    }
    let archive = temp.path().join("names.tar");
    let status = Command::new("tar")
        .arg("-cf")
        .arg(&archive)
        .arg("names")
        .current_dir(temp.path())
        .status()
        .expect("tar runs");
    assert!(status.success());
    let destination = temp.path().join("out");

    extract_archive(&archive, &destination).expect("plain names extract");

    for name in names {
        let unpacked = destination.join("names").join(name);
        assert_eq!(fs::read_to_string(unpacked).expect(name), name);
    }
}

/// The same colon in a zip, which `unzip -Z1` lists as it is.
#[cfg(unix)]
#[test]
fn extract_archive_accepts_a_colon_in_a_zip_member() {
    if !has("unzip") || !has("python3") {
        return;
    }
    let temp = TempDir::new().expect("temp");
    let archive = temp.path().join("colon.zip");
    let script = format!(
        "import zipfile\nwith zipfile.ZipFile({archive:?}, 'w') as z:\n    z.writestr('meeting 10:30.txt', 'notes')\n"
    );
    let status = Command::new("python3")
        .arg("-c")
        .arg(script)
        .status()
        .expect("python3 runs");
    assert!(status.success());
    let destination = temp.path().join("out");

    extract_archive(&archive, &destination).expect("a colon is a plain character");

    assert_eq!(
        fs::read_to_string(destination.join("meeting 10:30.txt")).expect("unpacked"),
        "notes"
    );
}
