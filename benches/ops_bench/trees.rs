//! Purpose: Source trees to copy, move and delete: many small files, one large file, a deep tree.

use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Files in the "many small" shape. Enough for the per-file cost to dominate,
/// few enough that building the tree once per iteration stays bearable.
const SMALL_FILES: usize = 500;

/// Bytes in each of them — a source file's order of magnitude.
const SMALL_SIZE: usize = 4 * 1_024;

/// The "one large" shape. Sixteen megabytes is a photograph or a short video,
/// and large enough that the copy is bandwidth rather than bookkeeping.
const LARGE_SIZE: usize = 16 * 1_024 * 1_024;

/// The shapes of work a benchmark copies, moves or deletes.
#[derive(Clone, Copy)]
pub enum Shape {
    ManySmall,
    OneLarge,
    DeepTree,
}

impl Shape {
    pub const ALL: [Self; 3] = [Self::ManySmall, Self::OneLarge, Self::DeepTree];

    /// The benchmark id criterion reports.
    pub const fn label(self) -> &'static str {
        match self {
            Self::ManySmall => "many_small",
            Self::OneLarge => "one_large",
            Self::DeepTree => "deep_tree",
        }
    }
}

/// A directory of `count` files of `size` bytes each.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn many_small(root: &Path, count: usize, size: usize) {
    fs::create_dir_all(root).expect("tree root");
    let payload = vec![b'x'; size];
    for index in 0..count {
        fs::write(root.join(format!("file-{index:05}.dat")), &payload).expect("tree file");
    }
}

/// A directory holding one file of `size` bytes.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn one_large(root: &Path, size: usize) {
    fs::create_dir_all(root).expect("tree root");
    fs::write(root.join("payload.bin"), vec![b'x'; size]).expect("large file");
}

/// A tree `depth` levels deep with `per_level` files at each.
///
/// Its own shape because a deep tree costs a `read_dir` and a `create_dir` per
/// level on top of the files, and a project checkout is mostly this.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn deep_tree(root: &Path, depth: usize, per_level: usize) {
    let mut path = root.to_path_buf();
    for level in 0..depth {
        path = path.join(format!("level-{level}"));
        fs::create_dir_all(&path).expect("nested directory");
        for index in 0..per_level {
            fs::write(path.join(format!("file-{index}.txt")), b"payload").expect("nested file");
        }
    }
}

/// Branches, nested folders, and files per leaf for the branching tree.
const BRANCH_COUNT: usize = 16;
const SUBDIRS_PER_BRANCH: usize = 4;
const FILES_PER_LEAF: usize = 8;

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn source_tree(shape: Shape) -> TempDir {
    let root = TempDir::new().expect("source tree");
    let path = root.path().join("tree");
    match shape {
        Shape::ManySmall => many_small(&path, SMALL_FILES, SMALL_SIZE),
        Shape::OneLarge => one_large(&path, LARGE_SIZE),
        Shape::DeepTree => deep_tree(&path, 10, 20),
    }
    root
}

/// A two-level branching directory tree (`512` files across `81` folders),
/// shaped like `node_modules` or `target` so parallel directory scanning and
/// batched unlinking run concurrently across worker threads.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
pub fn branching_project_tree() -> TempDir {
    let root = TempDir::new().expect("branching tree");
    let base = root.path().join("tree");
    for branch in 0..BRANCH_COUNT {
        let pkg = base.join(format!("pkg-{branch:02}"));
        for sub in 0..SUBDIRS_PER_BRANCH {
            let leaf = pkg.join(format!("mod-{sub:02}"));
            fs::create_dir_all(&leaf).expect("leaf directory");
            for item in 0..FILES_PER_LEAF {
                fs::write(leaf.join(format!("item-{item:02}.dat")), b"cache-entry")
                    .expect("leaf file");
            }
        }
    }
    root
}
