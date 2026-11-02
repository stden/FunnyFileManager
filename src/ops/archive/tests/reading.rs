//! Purpose: Listing checks before extraction, member types, and files that are not archives.

use super::*;

/// A member that would land outside the destination, or a link, is refused
/// before any tool writes — whichever tool listed it.
#[test]
fn extraction_rejects_parent_paths_and_links() {
    assert!(validate_listing("../outside", "-rw-r--r-- file").is_err());
    assert!(validate_listing("/outside", "-rw-r--r-- file").is_err());
    assert!(validate_listing("inside/../../outside", "-rw-r--r-- file").is_err());
    // `unzip` turns a DOS-made archive's `\` into `/`.
    assert!(validate_listing("..\\outside", "-rw-r--r-- file").is_err());
    #[cfg(windows)]
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

/// On Unix a colon or a backslash is an ordinary character of one name, and
/// `tar -tf` prints a tab, a backslash or (under `LANG=C`) a non-ASCII byte
/// as a backslash escape: none of them is a way out of the destination.
#[cfg(unix)]
#[test]
fn extraction_accepts_colons_and_tar_escapes_on_unix() {
    let names = "meeting 10:30.txt\nback\\\\slash\ntab\\there\ncaf\\303\\251.txt\nC:/inside";
    assert!(validate_listing(names, "-rw-r--r-- file").is_ok());
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

/// A `.zip` that is not a zip is refused with `unzip`'s reason, not with the
/// archive's own path in brackets, which is the first line it prints.
#[test]
fn a_zip_that_is_not_one_is_refused_with_the_reason() {
    if !tests_extract::has("unzip") {
        return;
    }
    let dir = TempDir::new().expect("dir");
    let fake = dir.path().join("fake.zip");
    fs::write(&fake, "not a zip\n").expect("write");

    let tested = test_archive(&fake).expect_err("not a zip");
    let extracted = extract_archive(&fake, &dir.path().join("out")).expect_err("not a zip");

    for error in [tested, extracted] {
        let text = error.to_string();
        assert!(
            text.contains("End-of-central-directory signature not found"),
            "{text}"
        );
        assert!(!text.contains('['), "{text}");
    }
}

/// A plain file is refused by name, with the accepted kinds spelled out.
#[test]
fn a_plain_file_is_not_an_archive_to_list_or_test() {
    let dir = TempDir::new().expect("dir");
    let plain = dir.path().join("readme.md");
    fs::write(&plain, "# hi\n").expect("write");
    let error = list_archive(&plain).expect_err("not an archive");
    assert!(
        error.to_string().contains(".zip"),
        "names what is accepted: {error}"
    );
    assert!(test_archive(&plain).is_err());
}
