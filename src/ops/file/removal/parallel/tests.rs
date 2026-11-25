//! Purpose: Unit tests for `parallel::Coordinator` worker scaling, deepest-first
//! directory removal, 16-entry batching, first-error retention, and stopped no-ops.

use std::fs;
use std::io;
use std::num::NonZeroUsize;
use std::path::Path;

use tempfile::TempDir;

use super::super::RemovalStep;
use super::{clamp_workers, remove_tree, worker_count, Coordinator, Task, BATCH_SIZE};
use crate::ops::file::tests_entry_removal::ForcedRemoval;
use crate::utils::FailedAt;

#[test]
fn worker_count_and_clamp_workers_respect_bounds() {
    assert!((2..=12).contains(&worker_count()));
    assert_eq!(clamp_workers(None), 4);
    assert_eq!(clamp_workers(NonZeroUsize::new(1)), 2);
    assert_eq!(clamp_workers(NonZeroUsize::new(2)), 2);
    assert_eq!(clamp_workers(NonZeroUsize::new(7)), 7);
    assert_eq!(clamp_workers(NonZeroUsize::new(12)), 12);
    assert_eq!(clamp_workers(NonZeroUsize::new(64)), 12);
}

fn build_four_level_tree(root: &Path) -> std::path::PathBuf {
    let deepest = root.join("l1").join("l2").join("l3");
    let sibling = root.join("l1_b").join("l2_b");
    fs::create_dir_all(&deepest).expect("mkdir deepest");
    fs::create_dir_all(&sibling).expect("mkdir sibling");
    fs::write(root.join("r.txt"), b"0").expect("write");
    fs::write(root.join("l1").join("a.txt"), b"1").expect("write");
    fs::write(deepest.join("deep.txt"), b"3").expect("write");
    fs::write(sibling.join("sib.txt"), b"2").expect("write");
    deepest
}

#[test]
fn four_level_tree_removes_deepest_directories_first() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path().join("tree");
    let deepest = build_four_level_tree(&root);

    let forced = ForcedRemoval::step(RemovalStep::RemoveDir);
    let err = remove_tree(&root, &mut |_| false).expect_err("deepest dir refused");
    drop(forced);

    let failed = FailedAt::of(&err).expect("FailedAt attached").path();
    assert_eq!(failed, deepest.as_path());
    assert!(root.join("l1").join("l2").exists());

    remove_tree(&root, &mut |_| false).expect("complete removal");
    assert!(!root.exists());
}

fn sample_types(dir: &Path) -> (fs::FileType, fs::FileType) {
    let (sub_ref, file_ref) = (dir.join("sub_ref"), dir.join("file_ref.txt"));
    fs::create_dir(&sub_ref).expect("mkdir");
    fs::write(&file_ref, b"x").expect("write");
    (
        fs::symlink_metadata(&sub_ref).expect("dir").file_type(),
        fs::symlink_metadata(&file_ref).expect("file").file_type(),
    )
}

#[test]
fn coordinator_batches_entries_at_sixteen() {
    let temp = TempDir::new().expect("tempdir");
    let (dir_ft, file_ft) = sample_types(temp.path());
    let coordinator = Coordinator::new(temp.path(), None);
    let (mut subdirs, mut batch) = (Vec::new(), Vec::new());
    for i in 0..20_usize {
        let (d, f) = (
            temp.path().join(format!("d{i}")),
            temp.path().join(format!("f{i}")),
        );
        coordinator.collect_entry(0, d, dir_ft, &mut subdirs, &mut batch);
        coordinator.collect_entry(0, f, file_ft, &mut subdirs, &mut batch);
    }
    coordinator.flush_scan(subdirs, batch);
    let queue = coordinator.lock();
    assert_eq!((queue.dirs.len(), queue.discovered), (21, 20));
    let lens: Vec<_> = queue
        .tasks
        .iter()
        .filter_map(|t| match t {
            Task::UnlinkFiles(files) => Some(files.len()),
            Task::ScanDir { .. } | Task::RemoveDirs(_) => None,
        })
        .collect();
    drop(queue);
    assert_eq!(lens, vec![BATCH_SIZE, 20 - BATCH_SIZE]);
}

#[test]
fn coordinator_retains_first_error() {
    let temp = TempDir::new().expect("tempdir");
    let coordinator = Coordinator::new(temp.path(), None);
    coordinator.fail(io::Error::new(io::ErrorKind::PermissionDenied, "first"));
    coordinator.fail(io::Error::other("second"));
    assert!(coordinator.is_stopped());
    assert!(coordinator.take_error().to_string().contains("first"));
}

#[test]
fn worker_tasks_become_noops_once_stopped() -> io::Result<()> {
    let temp = TempDir::new()?;
    let keep_dir = temp.path().join("keep_dir");
    let keep_file = keep_dir.join("keep.txt");
    fs::create_dir(&keep_dir)?;
    fs::write(&keep_file, b"keep")?;
    let ft = fs::symlink_metadata(&keep_file)?.file_type();

    let coordinator = Coordinator::new(temp.path(), None);
    coordinator.fail(io::Error::other("stop now"));
    coordinator.scan_dir(&keep_dir, 0)?;
    coordinator.unlink_files(vec![(keep_file.clone(), ft)])?;
    coordinator.remove_dirs(vec![keep_dir.clone()])?;

    assert!(keep_file.exists() && keep_dir.exists());
    Ok(())
}
