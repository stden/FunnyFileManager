//! Purpose: Save notifications and the local entrypoint touch only what they own.

use super::super::{temporary_path, update_saved};
use super::*;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn a_remote_root_cannot_be_used_as_an_upload_filename() {
    assert!(matches!(
        temporary_path(&RemotePath::root()),
        Err(RemoteError::InvalidInput("invalid remote path"))
    ));
}

#[test]
fn a_local_editor_does_not_write_through_the_remote_save_entrypoint() {
    let directory = tempfile::TempDir::new().unwrap();
    let path = directory.path().join("local");
    std::fs::write(&path, b"on disk").unwrap();
    let mut editor = Editor::from_content("local".to_owned(), path.clone(), "unsaved");
    save_direct(&mut editor).unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"on disk");
}

#[test]
fn save_metadata_without_a_remote_document_never_overwrites_local_bytes() {
    let directory = tempfile::TempDir::new().unwrap();
    let path = directory.path().join("local");
    std::fs::write(&path, b"on disk").unwrap();
    let mut editor = Editor::from_content("local".to_owned(), path.clone(), "unsaved");
    update_saved(&mut editor, b"server version", false);
    assert_eq!(editor.disk, Some(Fingerprint::of(b"server version")));
    assert_eq!(std::fs::read(path).unwrap(), b"on disk");
}

#[test]
fn a_save_notification_with_no_scratch_leaf_updates_only_remote_metadata() {
    let temporary = Arc::new(tempfile::TempDir::new().unwrap());
    let location = FileLocation::Remote {
        link: RemoteLink::new(Box::new(FakeRemoteFs::default())),
        path: RemotePath::root(),
    };
    let mut editor = Editor::from_content("remote".to_owned(), PathBuf::from("/"), "buffer");
    editor.remote = Some(Box::new(crate::models::RemoteDocument {
        location,
        fingerprint: Fingerprint::of(b"old"),
        temporary: Arc::clone(&temporary),
    }));
    update_saved(&mut editor, b"saved", true);
    let remote = editor.remote.as_ref().unwrap();
    assert_eq!(remote.fingerprint, Fingerprint::of(b"saved"));
    assert_eq!(std::fs::read_dir(temporary.path()).unwrap().count(), 0);
}
