use super::*;
use crate::models::{DirEntry, FileSize};
use crate::remote::{RemoteFs, RemoteMetadata, UnixMode, WriteMode};
use std::io::Read;

#[derive(Clone, Copy)]
enum Scenario {
    Changed,
    Permissions,
    Replacement,
    UnknownMode,
}

struct Host {
    inner: FakeRemoteFs,
    scenario: Scenario,
    stats: usize,
}

impl RemoteFs for Host {
    fn list(&mut self, path: &RemotePath) -> Result<Vec<DirEntry>, RemoteError> {
        self.inner.list(path)
    }
    fn stat(&mut self, path: &RemotePath) -> Result<RemoteMetadata, RemoteError> {
        self.stats += 1;
        let mut facts = self.inner.stat(path)?;
        if matches!(self.scenario, Scenario::UnknownMode) {
            facts.mode = None;
        }
        if matches!(self.scenario, Scenario::Changed) && self.stats == 2 {
            facts.size = FileSize::from_bytes(99);
        }
        Ok(facts)
    }
    fn upload(
        &mut self,
        path: &RemotePath,
        input: &mut dyn Read,
        mode: WriteMode,
    ) -> Result<u64, RemoteError> {
        self.inner.upload(path, input, mode)
    }
    fn chmod(&mut self, path: &RemotePath, mode: UnixMode) -> Result<(), RemoteError> {
        if matches!(self.scenario, Scenario::Permissions) {
            return Err(RemoteError::Failed("fixture chmod refused"));
        }
        self.inner.chmod(path, mode)
    }
    fn replace_file(
        &mut self,
        temporary: &RemotePath,
        target: &RemotePath,
    ) -> Result<(), RemoteError> {
        if matches!(self.scenario, Scenario::UnknownMode) {
            return self.inner.replace_file(temporary, target);
        }
        Err(RemoteError::Failed("fixture rename refused"))
    }
    fn remove_file(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        self.inner.remove_file(path)
    }
}

#[test]
fn failed_save_commit_cleans_staging_and_preserves_the_original_target() {
    for (scenario, expected) in [
        (Scenario::Changed, RemoteError::Conflict),
        (
            Scenario::Permissions,
            RemoteError::Failed("fixture chmod refused"),
        ),
        (
            Scenario::Replacement,
            RemoteError::Failed("fixture rename refused"),
        ),
    ] {
        assert_failed_save(scenario, expected);
    }
}

fn assert_failed_save(scenario: Scenario, expected: RemoteError) {
    let link = RemoteLink::new(Box::new(Host {
        inner: FakeRemoteFs::with_files(&[("/file", 3)]),
        scenario,
        stats: 0,
    }));
    let path = RemotePath::parse("/file").expect("path");
    let source = FileLocation::Remote {
        link: link.clone(),
        path: path.clone(),
    };
    assert_eq!(
        save(&source, Fingerprint::of(b"old"), b"replacement", true),
        Err(expected)
    );
    assert_target(&link, &path, 3);
}

fn assert_target(link: &RemoteLink, path: &RemotePath, size: u64) {
    let entries = link
        .lock()
        .list(&RemotePath::root())
        .expect("remaining files");
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        ["file"]
    );
    assert_eq!(
        link.lock()
            .stat(path)
            .expect("original target")
            .size
            .bytes(),
        size
    );
}

#[test]
fn a_save_succeeds_when_the_server_does_not_report_permissions() {
    let link = RemoteLink::new(Box::new(Host {
        inner: FakeRemoteFs::with_files(&[("/file", 3)]),
        scenario: Scenario::UnknownMode,
        stats: 0,
    }));
    let path = RemotePath::parse("/file").expect("path");
    let source = FileLocation::Remote {
        link: link.clone(),
        path: path.clone(),
    };
    assert_eq!(
        save(&source, Fingerprint::of(b"old"), b"updated", true),
        Ok(())
    );
    assert_target(&link, &path, 7);
}
