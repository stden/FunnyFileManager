//! Purpose: Only owned temporary fixtures are eligible; fake caches and downloads stay outside.
use super::{Case, TempCleanupJobs};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

pub struct Fixture {
    pub(super) jobs: TempCleanupJobs,
    pub(super) directory: PathBuf,
    pub(super) home: PathBuf,
    pub(super) old: PathBuf,
    pub(super) fresh: PathBuf,
    pub(super) retained_directory: PathBuf,
    pub(super) outside: Vec<PathBuf>,
    old_content: OldContent,
}

enum OldContent {
    Original,
    Changed,
}

impl Fixture {
    pub(super) fn new(case: &Case) -> Self {
        let directory = case.root.path().join("temporary-files");
        let home = case.root.path().join("fake-home");
        let old = directory.join("old.tmp");
        let fresh = directory.join("fresh.tmp");
        let retained_directory = directory.join("empty-folder");
        fs::create_dir_all(&retained_directory).unwrap();
        fs::write(&old, [b'o'; 128]).unwrap();
        age(&old);
        fs::write(&fresh, b"fresh temporary content").unwrap();
        let outside = outside_files(&home);
        Self {
            jobs: TempCleanupJobs::default(),
            directory,
            home,
            old,
            fresh,
            retained_directory,
            outside,
            old_content: OldContent::Original,
        }
    }

    pub(super) fn change_old_file(&mut self) {
        fs::write(&self.old, b"modified after preview").unwrap();
        self.old_content = OldContent::Changed;
    }

    pub(super) fn assert_old_content(&self) {
        let expected: &[u8] = match self.old_content {
            OldContent::Original => &[b'o'; 128],
            OldContent::Changed => b"modified after preview",
        };
        assert_eq!(fs::read(&self.old).unwrap(), expected);
    }

    pub(super) fn assert_protected_content(&self) {
        assert!(self.directory.is_dir(), "temporary root must remain");
        assert!(
            self.retained_directory.is_dir(),
            "child directories must remain"
        );
        assert_eq!(fs::read(&self.fresh).unwrap(), b"fresh temporary content");
        for outside in &self.outside {
            assert_eq!(fs::read(outside).unwrap(), b"old outside content");
        }
    }
}

fn outside_files(home: &Path) -> Vec<PathBuf> {
    let files = [
        home.join(".cache/application.cache"),
        home.join("Downloads/download.bin"),
    ];
    for file in &files {
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, b"old outside content").unwrap();
        age(file);
    }
    files.into()
}

fn age(path: &Path) {
    let old = SystemTime::now()
        .checked_sub(Duration::from_hours(192))
        .unwrap();
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(old)
        .unwrap();
}
