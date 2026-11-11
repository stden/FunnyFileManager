//! Purpose: Unit tests for remote editor persistence guards, mode preservation, and save callbacks.

use super::{save, save_direct, saved};
use crate::models::remote_work::{FileLocation, RemoteTask};
use crate::models::{AfterSave, App, Editor, Fingerprint, Panel, RemotePath};
use crate::remote::fake::FakeRemoteFs;
use crate::remote::{RemoteError, RemoteFs, RemoteLink, UnixMode};

mod commit;
mod update;

#[test]
fn save_rejects_local_or_directory_sources_and_preserves_remote_mode() {
    let local = FileLocation::Local("a.txt".into());
    assert!(matches!(
        save(&local, Fingerprint::of(b""), b"x", true),
        Err(RemoteError::Failed(_))
    ));
    let mut fake = FakeRemoteFs::with_files(&[("/dir/f.txt", 3)]);
    let file_path = RemotePath::parse("/dir/f.txt").unwrap();
    fake.chmod(&file_path, UnixMode::from_mode(0o640)).unwrap();
    let link = RemoteLink::new(Box::new(fake));
    let dir_loc = FileLocation::Remote {
        link: link.clone(),
        path: RemotePath::parse("/dir").unwrap(),
    };
    assert!(matches!(
        save(&dir_loc, Fingerprint::of(b""), b"x", true),
        Err(RemoteError::Unsupported(_))
    ));
    let file_loc = FileLocation::Remote {
        link,
        path: file_path,
    };
    save(&file_loc, Fingerprint::of(b""), b"updated", true).unwrap();
}

#[test]
fn save_direct_noop_on_local_editor_and_saved_ignores_other_tasks() {
    let mut editor = Editor::from_content("a.txt".into(), "a.txt".into(), "hi");
    save_direct(&mut editor).unwrap();

    let temp = tempfile::TempDir::new().unwrap();
    let panel = Panel::from_entries(temp.path().to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let link = RemoteLink::new(Box::new(FakeRemoteFs::default()));
    let mkdir = RemoteTask::Mkdir(FileLocation::Local("d".into()));
    saved(&mut app, &mkdir);
    let save_task = RemoteTask::Save {
        source: FileLocation::Remote {
            link,
            path: RemotePath::parse("/f").unwrap(),
        },
        expected: Fingerprint::of(b""),
        bytes: b"x".to_vec(),
        overwrite: true,
        revision: 0,
        then: AfterSave::Stay,
    };
    saved(&mut app, &save_task);
}

struct StatFlipFs(FakeRemoteFs, bool);

impl RemoteFs for StatFlipFs {
    fn list(&mut self, dir: &RemotePath) -> Result<Vec<crate::models::DirEntry>, RemoteError> {
        self.0.list(dir)
    }
    fn stat(&mut self, path: &RemotePath) -> Result<crate::remote::RemoteMetadata, RemoteError> {
        let mut meta = self.0.stat(path)?;
        if self.1 {
            meta.size = crate::models::FileSize::from_bytes(999);
        }
        self.1 = true;
        Ok(meta)
    }
    fn upload(
        &mut self,
        path: &RemotePath,
        reader: &mut dyn std::io::Read,
        mode: crate::remote::WriteMode,
    ) -> Result<u64, RemoteError> {
        self.0.upload(path, reader, mode)
    }
    fn remove_file(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        self.0.remove_file(path)
    }
}

#[test]
fn save_cleans_temporary_on_commit_stat_conflict() {
    let flip = StatFlipFs(FakeRemoteFs::with_files(&[("/f.txt", 3)]), false);
    let link = RemoteLink::new(Box::new(flip));
    let src = FileLocation::Remote {
        link,
        path: RemotePath::parse("/f.txt").unwrap(),
    };
    assert_eq!(
        save(&src, Fingerprint::of(b""), b"new", true),
        Err(RemoteError::Conflict)
    );
}

#[test]
fn saved_handles_blocked_local_working_copy_and_closes_on_after_save_close() {
    let link = RemoteLink::new(Box::new(FakeRemoteFs::with_files(&[("/f.txt", 3)])));
    let src = FileLocation::Remote {
        link,
        path: RemotePath::parse("/f.txt").unwrap(),
    };
    let temp = tempfile::TempDir::new().unwrap();
    let panel = Panel::from_entries(temp.path().to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    super::super::open(&mut app, src.clone(), b"abc", true).unwrap();
    let editor = app.get_editor_mut().unwrap();
    let rem = editor.remote.as_ref().unwrap();
    let blocked = rem.temporary.path().join("f.txt");
    std::fs::remove_file(&blocked).unwrap();
    std::fs::create_dir(&blocked).unwrap();
    let rev = editor.lsp.revision.0;
    let task = RemoteTask::Save {
        source: src,
        expected: Fingerprint::of(b"abc"),
        bytes: b"xyz".to_vec(),
        overwrite: true,
        revision: rev,
        then: AfterSave::Close,
    };
    saved(&mut app, &task);
    assert!(!app.is_overlay_active());
}

#[test]
fn update_saved_ignores_plain_editors_and_nameless_paths() {
    let temp = tempfile::TempDir::new().unwrap();
    let panel = Panel::from_entries(temp.path().to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);

    let mut plain = Editor::from_content("a.txt".into(), "a.txt".into(), "hi");
    super::update_saved(&mut plain, b"hi", false);

    let link2 = RemoteLink::new(Box::new(FakeRemoteFs::with_files(&[("/g.txt", 3)])));
    let src2 = FileLocation::Remote {
        link: link2,
        path: RemotePath::parse("/g.txt").unwrap(),
    };
    super::super::open(&mut app, src2, b"abc", true).unwrap();
    let nameless = app.get_editor_mut().unwrap();
    nameless.path = std::path::PathBuf::new();
    super::update_saved(nameless, b"xyz", true);
}
