//! Purpose: `Given` steps that lay out a scenario on disk: the temp root,
//! files and subdirectories in it, symlinks, out-of-root targets and modes.

use crate::world::McpWorld;
use cucumber::given;
use std::fs;
use tempfile::TempDir;

#[given("a temporary directory")]
#[expect(
    clippy::expect_used,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_temp_dir(w: &mut McpWorld) {
    w.temp_dir = Some(TempDir::new().expect("create temp dir"));
}

#[given(expr = "a file {string} with content {string}")]
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_file(w: &mut McpWorld, name: String, content: String) {
    let path = w.temp_path().join(&name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    let unescaped = content.replace("\\n", "\n");
    fs::write(&path, &unescaped).unwrap_or_else(|e| panic!("write {}: {}", path.display(), e));
}

#[given(expr = "a subdirectory {string}")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_subdir(w: &mut McpWorld, name: String) {
    let path = w.temp_path().join(&name);
    fs::create_dir_all(&path).unwrap_or_else(|e| panic!("mkdir {}: {}", path.display(), e));
}

#[given(expr = "a symlink {string} pointing to {string}")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_symlink(w: &mut McpWorld, link: String, target: String) {
    let link_path = w.temp_path().join(&link);
    let target_path = w.temp_path().join(&target);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target_path, &link_path)
        .unwrap_or_else(|e| panic!("symlink {}: {}", link_path.display(), e));
    #[cfg(windows)]
    {
        let result = if target_path.is_dir() {
            std::os::windows::fs::symlink_dir(&target_path, &link_path)
        } else {
            std::os::windows::fs::symlink_file(&target_path, &link_path)
        };
        result.unwrap_or_else(|e| panic!("symlink {}: {}", link_path.display(), e));
    }
}

#[given(expr = "a file {string} with content {string} outside the root")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_outside_file(w: &mut McpWorld, name: String, content: String) {
    let path = w.outside_path().join(&name);
    fs::write(&path, content.replace("\\n", "\n"))
        .unwrap_or_else(|e| panic!("write {}: {}", path.display(), e));
}

/// A link *inside* the root naming a file outside it — what a caller plants
/// when the confinement stops at the directory a walk starts from.
#[given(expr = "a symlink {string} pointing outside the root at {string}")]
#[expect(
    clippy::panic,
    reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
)]
async fn given_outside_symlink(w: &mut McpWorld, link: String, target: String) {
    let link_path = w.temp_path().join(&link);
    let target_path = w.outside_path().join(&target);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target_path, &link_path)
        .unwrap_or_else(|e| panic!("symlink {}: {}", link_path.display(), e));
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&target_path, &link_path)
        .unwrap_or_else(|e| panic!("symlink {}: {}", link_path.display(), e));
}

#[given(expr = "the file {string} has mode {string}")]
#[cfg_attr(
    unix,
    expect(
        clippy::expect_used,
        clippy::panic,
        reason = "BDD steps fail by panicking; harness = false puts them outside the test allowances"
    )
)]
async fn given_mode(w: &mut McpWorld, name: String, mode: String) {
    let path = w.temp_path().join(&name);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let bits = u32::from_str_radix(&mode, 8).expect("octal mode");
        fs::set_permissions(&path, fs::Permissions::from_mode(bits))
            .unwrap_or_else(|e| panic!("chmod {}: {}", path.display(), e));
    }
    #[cfg(not(unix))]
    drop((path, mode));
}
