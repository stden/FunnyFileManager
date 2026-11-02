//! Purpose: Build Git histories whose remotes stay inside a temporary directory.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(super) struct Fixture {
    _temp: tempfile::TempDir,
    pub(super) root: PathBuf,
    pub(super) seed: PathBuf,
    pub(super) remote: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let temp = tempfile::TempDir::new().expect("temporary directory");
        let root = temp.path().join("workspace");
        let seed = temp.path().join("seed");
        let remote = temp.path().join("origin.git");
        for path in [&root, &seed] {
            fs::create_dir(path).expect("directory");
        }
        git(
            temp.path(),
            &["init", "--bare", "--initial-branch=main", text(&remote)],
        );
        git(&seed, &["init", "--initial-branch=main"]);
        fs::write(seed.join("data.txt"), "original\n").expect("file");
        commit(&seed, "initial");
        git(&seed, &["push", text(&remote), "main"]);
        Self {
            _temp: temp,
            root,
            seed,
            remote,
        }
    }

    pub(super) fn clone_repo(&self, name: &str) -> PathBuf {
        let path = self.root.join(name);
        git(&self.root, &["clone", text(&self.remote), text(&path)]);
        git(&path, &["config", "core.autocrlf", "false"]);
        path
    }

    pub(super) fn advance(&self) {
        fs::write(self.seed.join("data.txt"), "remote update\n").expect("file");
        commit(&self.seed, "remote change");
        git(&self.seed, &["push", text(&self.remote), "main"]);
    }
}

pub(super) fn text(path: &Path) -> &str {
    path.to_str().expect("fixture path")
}

pub(super) fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", repo.join("absent-global-config"))
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("Git");
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

pub(super) fn commit(repo: &Path, message: &str) {
    git(repo, &["add", "."]);
    git(repo, &["commit", "-m", message]);
}
