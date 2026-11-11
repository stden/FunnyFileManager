//! Purpose: Verify downloaded checksums, traversal rejection, and safe archive extraction.

use super::*;
use crate::lsp::install::environment::System;
use tempfile::TempDir;

fn archive(kind: tar::EntryType) -> Vec<u8> {
    let mut archive = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(4);
    header.set_mode(0o755);
    header.set_entry_type(kind);
    header.set_cksum();
    archive
        .append_data(&mut header, "bin/server", &b"data"[..])
        .unwrap();
    let tar = archive.into_inner().unwrap();
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(&tar).unwrap();
    gzip.finish().unwrap()
}

#[test]
fn verified_archives_extract_files_into_the_managed_directory() {
    let root = TempDir::new().unwrap();
    let data = archive(tar::EntryType::Regular);
    let path = root.path().join("download");
    fs::write(&path, &data).unwrap();
    let stop = AtomicBool::new(false);
    verify(&path, &storage::digest(&data), &stop).unwrap();
    assert_eq!(
        verify(&path, &"0".repeat(64), &stop),
        Err(InstallError::Integrity)
    );
    unpack(&*data, root.path(), &stop).unwrap();
    assert_eq!(fs::read(root.path().join("bin/server")).unwrap(), b"data");
}

#[test]
fn links_invalid_downloads_and_cancelled_extraction_are_rejected() {
    let root = TempDir::new().unwrap();
    let stop = AtomicBool::new(false);
    let data = archive(tar::EntryType::Symlink);
    assert_eq!(
        unpack(&*data, root.path(), &stop),
        Err(InstallError::Archive)
    );
}

#[test]
fn invalid_download_parameters_are_rejected_before_network_access() {
    let root = TempDir::new().unwrap();
    let stop = AtomicBool::new(false);
    assert_eq!(
        download(
            "http://example.com/server.tar.gz",
            &"0".repeat(64),
            root.path(),
            &stop,
            &System
        ),
        Err(InstallError::Recipe)
    );
    assert_eq!(
        download(
            "https://example.com/server.tar.gz",
            "bad",
            root.path(),
            &stop,
            &System
        ),
        Err(InstallError::Recipe)
    );
}

#[test]
fn invalid_archives_and_cancellation_stop_extraction() {
    let root = TempDir::new().unwrap();
    let stop = AtomicBool::new(false);
    assert!(unpack(&b"bad archive"[..], root.path(), &stop).is_err());
    assert_eq!(
        unpack(
            &*archive(tar::EntryType::Regular),
            root.path(),
            &AtomicBool::new(true)
        ),
        Err(InstallError::Cancelled)
    );
}

#[test]
fn a_traversal_member_cannot_write_outside_the_managed_directory() {
    let root = TempDir::new().unwrap();
    let managed = root.path().join("managed");
    fs::create_dir(&managed).unwrap();
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut archive = tar::Builder::new(gzip);
    let mut header = tar::Header::new_gnu();
    header.as_mut_bytes()[..10].copy_from_slice(b"../outside");
    header.set_size(4);
    header.set_mode(0o644);
    header.set_cksum();
    archive.append(&header, &b"data"[..]).unwrap();
    let data = archive.into_inner().unwrap().finish().unwrap();
    assert_eq!(
        unpack(&*data, &managed, &AtomicBool::new(false)),
        Err(InstallError::Archive)
    );
    assert!(!root.path().join("outside").exists());
    assert_eq!(fs::read_dir(managed).unwrap().count(), 0);
}

#[test]
fn curl_must_bound_downloads_without_a_content_length() {
    let root = TempDir::new().unwrap();
    let mut packages = crate::lsp::install::fixtures::Packages::new();
    for version in ["curl 8.3.0", "curl 7.88.1", "invalid"] {
        packages.curl_version = version.into();
        assert!(matches!(
            download(
                "https://example.com/server.tar.gz",
                &storage::digest(&packages.archive),
                root.path(),
                &AtomicBool::new(false),
                &packages
            ),
            Err(InstallError::Missing(_))
        ));
    }
    assert!(packages
        .calls
        .borrow()
        .iter()
        .all(|args| args == &["--version"]));
    assert!(!root.path().join("server.tar.gz").exists());
}

#[test]
fn directory_entries_extract_and_oversized_archives_are_rejected() {
    let root = TempDir::new().unwrap();
    let stop = AtomicBool::new(false);

    let mut dir_tar = tar::Builder::new(Vec::new());
    let mut dir_hdr = tar::Header::new_gnu();
    dir_hdr.set_size(0);
    dir_hdr.set_mode(0o755);
    dir_hdr.set_entry_type(tar::EntryType::Directory);
    dir_hdr.set_cksum();
    dir_tar
        .append_data(&mut dir_hdr, "nested/dir", &b""[..])
        .unwrap();
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&dir_tar.into_inner().unwrap()).unwrap();
    unpack(&*gz.finish().unwrap(), root.path(), &stop).unwrap();
    assert!(root.path().join("nested/dir").is_dir());

    let mut raw = vec![0u8; 1536];
    let mut huge = tar::Header::new_gnu();
    huge.set_path("huge.bin").unwrap();
    huge.set_size(2_147_483_649);
    huge.set_mode(0o644);
    huge.set_entry_type(tar::EntryType::Regular);
    huge.set_cksum();
    raw[..512].copy_from_slice(huge.as_bytes());
    let mut huge_gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    huge_gz.write_all(&raw).unwrap();
    assert_eq!(
        unpack(&*huge_gz.finish().unwrap(), root.path(), &stop),
        Err(InstallError::Archive)
    );
}
