//! Unit tests exercising `SftpFs` operations and error cases through `FAKE_SFTP_OPS`.

use super::{FakeBackend, FAKE_SFTP_OPS};
use crate::models::{EntryKind, RemotePath};
use crate::remote::sftp::{metadata, SftpFs};
use crate::remote::{RemoteError, RemoteFs, UnixMode, WriteMode};
use std::path::PathBuf;

fn fake_fs() -> SftpFs<FakeBackend, super::FakeFile> {
    SftpFs {
        sftp: FakeBackend::default(),
        ops: &FAKE_SFTP_OPS,
    }
}

#[test]
fn sftp_fs_round_trips_crud_special_files_and_rename_clashes() {
    let mut fs = fake_fs();
    let dir = RemotePath::parse("/work").expect("dir");
    let a = RemotePath::parse("/work/a.txt").expect("a");
    let b = RemotePath::parse("/work/b.txt").expect("b");
    fs.mkdir(&dir).expect("mkdir");
    assert_eq!(fs.stat(&dir).expect("stat").kind, EntryKind::Directory);
    assert!(matches!(
        fs.download(&dir, &mut Vec::new()),
        Err(RemoteError::Unsupported("reading special files"))
    ));
    assert_eq!(
        fs.upload(&a, &mut b"abc".as_slice(), WriteMode::CreateNew),
        Ok(3)
    );
    assert_eq!(fs.list(&dir).expect("list").len(), 2);
    assert!(matches!(
        fs.rename(&a, &a),
        Err(RemoteError::AlreadyExists(_))
    ));
    fs.rename(&a, &b).expect("rename");
    fs.chmod(&b, UnixMode::from_mode(0o640)).expect("chmod");
    assert_eq!(fs.download(&b, &mut Vec::new()), Ok(8));
    assert_eq!(
        fs.upload(&a, &mut b"xy".as_slice(), WriteMode::Overwrite),
        Ok(2)
    );
    fs.replace_file(&a, &b).expect("replace");
    fs.remove_file(&b).expect("unlink");
    fs.rmdir(&dir).expect("rmdir");
}

fn stat_with(mode: u32) -> ssh2::FileStat {
    let mut stat = metadata::empty_stat();
    stat.perm = Some(mode);
    stat
}

#[test]
fn sftp_fs_resolves_symlink_targets_and_rejects_nameless_paths() {
    let mut fs = fake_fs();
    let dir = RemotePath::parse("/work").expect("dir");
    let link = stat_with(0o120_777);
    for (p, target) in [
        ("/ldir", Ok(stat_with(0o040_755))),
        ("/lfile", Ok(stat_with(0o100_644))),
        ("/lbroken", Err(2)),
    ] {
        let key = PathBuf::from(p);
        fs.sftp
            .entries
            .lock()
            .expect("lock")
            .insert(key.clone(), (link.clone(), Vec::new()));
        fs.sftp.targets.lock().expect("lock").insert(key, target);
    }
    assert_eq!(fs.list(&dir).expect("links").len(), 3);
    fs.sftp
        .entries
        .lock()
        .expect("lock")
        .insert(PathBuf::new(), (link, Vec::new()));
    assert!(matches!(
        fs.list(&dir),
        Err(RemoteError::Failed("SFTP returned a non-UTF-8 file name"))
    ));
}

#[test]
fn sftp_fs_propagates_rename_stat_errors_and_close_failures() {
    let mut fs = fake_fs();
    let a = RemotePath::parse("/work/a.txt").expect("a");
    let b = RemotePath::parse("/work/b.txt").expect("b");
    assert_eq!(
        fs.upload(&a, &mut b"ok".as_slice(), WriteMode::Overwrite),
        Ok(2)
    );
    *fs.sftp.fail_code.lock().expect("lock") = Some(3);
    assert!(fs.rename(&a, &b).is_err());
    *fs.sftp.fail_code.lock().expect("lock") = None;
    *fs.sftp.fail_close.lock().expect("lock") = true;
    assert!(fs
        .upload(&a, &mut b"z".as_slice(), WriteMode::Overwrite)
        .is_err());
    assert!(fs.download(&a, &mut Vec::new()).is_err());
}
