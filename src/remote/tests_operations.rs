//! Purpose: Remote files support streaming transfers, rename, permissions and removal.

use super::fake::FakeRemoteFs;
use super::{RemoteFs, UnixMode, WriteMode};
use crate::models::RemotePath;

fn path(text: &str) -> RemotePath {
    RemotePath::parse(text).expect("remote path")
}

#[test]
fn remote_file_lifecycle_preserves_contents_and_permissions() {
    let mut fs = FakeRemoteFs::default();
    let dir = path("/work");
    let first = path("/work/first.txt");
    let second = path("/work/second.txt");
    fs.mkdir(&dir).expect("mkdir");
    let content = b"hello\nremote\n";
    fs.upload(&first, &mut content.as_slice(), WriteMode::CreateNew)
        .expect("upload");
    let mut downloaded = Vec::new();
    fs.download(&first, &mut downloaded).expect("download");
    assert_eq!(downloaded, content);
    assert_eq!(fs.stat(&first).expect("stat").size.bytes(), 13);
    let mode = UnixMode::parse("0640").expect("mode");
    fs.chmod(&first, mode).expect("chmod");
    assert_eq!(fs.stat(&first).expect("stat").mode, Some(mode));
    fs.rename(&first, &second).expect("rename");
    assert!(fs.stat(&first).is_err());
    assert!(fs.rmdir(&dir).is_err(), "a nonempty directory must survive");
    fs.remove_file(&second).expect("remove file");
    fs.rmdir(&dir).expect("rmdir");
    assert_eq!(fs.list(&RemotePath::root()).expect("root").len(), 0);
}

#[test]
fn create_new_and_rename_do_not_destroy_an_existing_destination() {
    let mut fs = FakeRemoteFs::default();
    let first = path("/first");
    let second = path("/second");
    for file in [&first, &second] {
        fs.upload(file, &mut b"original".as_slice(), WriteMode::CreateNew)
            .expect("upload");
    }
    assert!(fs
        .upload(&first, &mut b"replacement".as_slice(), WriteMode::CreateNew)
        .is_err());
    assert!(fs.rename(&first, &second).is_err());
    let mut bytes = Vec::new();
    fs.download(&first, &mut bytes).expect("still there");
    assert_eq!(bytes, b"original");
    fs.upload(&first, &mut b"edited".as_slice(), WriteMode::Overwrite)
        .expect("overwrite");
    bytes.clear();
    fs.download(&first, &mut bytes).expect("edited");
    assert_eq!(bytes, b"edited");
}

#[test]
fn unix_modes_reject_invalid_or_non_octal_input() {
    for invalid in ["", "888", "10000", "-755", "755\nSITE DELE /file"] {
        assert!(UnixMode::parse(invalid).is_err(), "{invalid}");
    }
    for valid in ["0", "644", "0755", "4755", "7777"] {
        assert_eq!(
            UnixMode::parse(valid).expect("mode").bits(),
            u32::from_str_radix(valid, 8).expect("octal")
        );
    }
}

#[test]
fn listing_metadata_retains_linux_permission_bits() {
    let entry = crate::models::DirEntry {
        attributes: crate::models::Attributes::from_bits(0o100_640),
        ..crate::models::DirEntry::default()
    };
    assert_eq!(
        super::RemoteMetadata::from_entry(&entry).mode,
        Some(UnixMode::from_mode(0o640))
    );
}
