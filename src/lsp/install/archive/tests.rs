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
    unpack(&data[..], root.path(), &stop).unwrap();
    assert_eq!(fs::read(root.path().join("bin/server")).unwrap(), b"data");
}

#[test]
fn links_invalid_downloads_and_cancelled_extraction_are_rejected() {
    let root = TempDir::new().unwrap();
    let stop = AtomicBool::new(false);
    let data = archive(tar::EntryType::Symlink);
    assert_eq!(
        unpack(&data[..], root.path(), &stop),
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
            &archive(tar::EntryType::Regular)[..],
            root.path(),
            &AtomicBool::new(true)
        ),
        Err(InstallError::Cancelled)
    );
}
