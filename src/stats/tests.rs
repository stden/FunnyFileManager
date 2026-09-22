//! Unit tests: measuring a directory, and the limits that keep it safe.

use super::*;
use std::fs;
use tempfile::TempDir;

/// A tree built from `(relative path, size in bytes)`.
fn tree(files: &[(&str, u64)]) -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    for (path, size) in files {
        let full = dir.path().join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("parents");
        }
        fs::write(&full, vec![b'x'; *size as usize]).expect("file");
    }
    dir
}

fn child(stats: &DirectoryStats, name: &str) -> Option<ChildSize> {
    stats.children.iter().find(|c| c.name == name).cloned()
}

fn kind(stats: &DirectoryStats, kind: ContentKind) -> Option<KindTotal> {
    stats.by_kind.iter().copied().find(|k| k.kind == kind)
}

#[test]
fn an_empty_directory_measures_zero() {
    let dir = TempDir::new().expect("temp dir");
    let stats = analyse(dir.path());

    assert_eq!(stats.total.bytes(), 0);
    assert_eq!(stats.files, 0);
    assert_eq!(stats.directories, 0);
    assert!(stats.children.is_empty());
    assert!(stats.by_kind.is_empty());
    assert!(stats.coverage.is_complete());
}

/// A directory that cannot be read answers zero rather than refusing: one
/// denied directory must not cost the user the whole view.
#[test]
fn a_missing_directory_answers_rather_than_failing() {
    let stats = analyse(Path::new("/definitely/not/here"));
    assert_eq!(stats.files, 0);
    assert!(stats.coverage.is_complete());
}

/// A subdirectory's size is everything beneath it, not what sits in it.
#[test]
fn a_subdirectory_carries_the_weight_of_its_whole_subtree() {
    let dir = tree(&[
        ("big/nested/deep/a.bin", 3000),
        ("big/b.bin", 1000),
        ("small/c.bin", 10),
    ]);
    let stats = analyse(dir.path());

    assert_eq!(child(&stats, "big").unwrap().size.bytes(), 4000);
    assert_eq!(child(&stats, "big").unwrap().files, 2);
    assert_eq!(child(&stats, "small").unwrap().size.bytes(), 10);
    assert_eq!(stats.total.bytes(), 4010);
    assert_eq!(stats.files, 3, "files anywhere in the tree are counted");
}

/// The point of the view: the biggest subdirectory is the first thing shown.
#[test]
fn subdirectories_are_ordered_by_size() {
    let dir = tree(&[("a/x", 10), ("b/x", 5000), ("c/x", 300)]);
    let names: Vec<String> = analyse(dir.path())
        .children
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(names, ["b", "c", "a"]);
}

#[test]
fn files_at_the_top_level_count_toward_the_total() {
    let dir = tree(&[("loose.bin", 700), ("sub/inner.bin", 300)]);
    let stats = analyse(dir.path());

    assert_eq!(stats.total.bytes(), 1000);
    assert_eq!(stats.files, 2);
    assert_eq!(stats.directories, 1);
}

// ─── content kinds ──────────────────────────────────────────────────────────

#[test]
fn a_file_is_categorised_by_its_extension() {
    assert_eq!(ContentKind::of("main.rs"), ContentKind::Source);
    assert_eq!(ContentKind::of("notes.md"), ContentKind::Document);
    assert_eq!(ContentKind::of("photo.JPG"), ContentKind::Image);
    assert_eq!(ContentKind::of("song.flac"), ContentKind::Audio);
    assert_eq!(ContentKind::of("clip.mkv"), ContentKind::Video);
    assert_eq!(ContentKind::of("bundle.tar.gz"), ContentKind::Archive);
    assert_eq!(ContentKind::of("libthing.so"), ContentKind::Executable);
    assert_eq!(ContentKind::of("data.json"), ContentKind::Data);
}

/// No extension, or one nobody has heard of, is `Other` — not a guess.
#[test]
fn an_unknown_or_absent_extension_is_other() {
    assert_eq!(ContentKind::of("Makefile"), ContentKind::Other);
    assert_eq!(ContentKind::of("archive.zzzz"), ContentKind::Other);
    assert_eq!(ContentKind::of(""), ContentKind::Other);
    // A leading dot is a hidden file, not an extension.
    assert_eq!(ContentKind::of(".gitignore"), ContentKind::Other);
}

#[test]
fn the_breakdown_totals_each_category_and_orders_by_size() {
    let dir = tree(&[
        ("a.rs", 100),
        ("b.rs", 200),
        ("doc.md", 50),
        ("media/clip.mp4", 9000),
    ]);
    let stats = analyse(dir.path());

    assert_eq!(kind(&stats, ContentKind::Source).unwrap().size.bytes(), 300);
    assert_eq!(kind(&stats, ContentKind::Source).unwrap().files, 2);
    assert_eq!(
        kind(&stats, ContentKind::Document).unwrap().size.bytes(),
        50
    );
    assert_eq!(
        stats.by_kind.first().map(|k| k.kind),
        Some(ContentKind::Video),
        "the largest category leads: {:?}",
        stats.by_kind
    );
}

/// Every byte lands in exactly one category, so the breakdown adds up to the
/// total — otherwise the view quietly loses files.
#[test]
fn the_categories_account_for_every_byte() {
    let dir = tree(&[
        ("a.rs", 100),
        ("b.png", 250),
        ("c.unknown", 7),
        ("sub/d.zip", 1000),
    ]);
    let stats = analyse(dir.path());

    let summed: u64 = stats.by_kind.iter().map(|k| k.size.bytes()).sum();
    assert_eq!(summed, stats.total.bytes());
    let counted: usize = stats.by_kind.iter().map(|k| k.files).sum();
    assert_eq!(counted, stats.files);
}

// ─── largest files ──────────────────────────────────────────────────────────

#[test]
fn the_largest_files_are_named_with_their_path() {
    let dir = tree(&[("small.bin", 1), ("deep/huge.bin", 9999)]);
    let stats = analyse(dir.path());

    let biggest = stats.largest.first().expect("a largest file");
    assert_eq!(biggest.size.bytes(), 9999);
    assert!(
        biggest.path.contains("huge.bin"),
        "the path must lead to it: {}",
        biggest.path
    );
    assert!(biggest.path.contains("deep"), "including its directory");
}

#[test]
fn the_largest_list_is_capped_and_ordered() {
    let files: Vec<(String, u64)> = (0u64..40)
        .map(|i| (format!("f{i:02}.bin"), i * 10 + 1))
        .collect();
    let borrowed: Vec<(&str, u64)> = files.iter().map(|(n, s)| (n.as_str(), *s)).collect();
    let dir = tree(&borrowed);
    let stats = analyse(dir.path());

    assert_eq!(stats.largest.len(), LARGEST_SHOWN);
    assert!(
        stats.largest.windows(2).all(|w| w[0].size >= w[1].size),
        "largest first: {:?}",
        stats.largest
    );
    assert_eq!(
        stats.largest[0].size.bytes(),
        391,
        "and it really is the biggest"
    );
}

/// Only so many subdirectories are shown, and they are the biggest ones.
#[test]
fn the_subdirectory_list_is_capped() {
    let files: Vec<(String, u64)> = (0u64..30)
        .map(|i| (format!("d{i:02}/x.bin"), i + 1))
        .collect();
    let borrowed: Vec<(&str, u64)> = files.iter().map(|(n, s)| (n.as_str(), *s)).collect();
    let dir = tree(&borrowed);
    let stats = analyse(dir.path());

    assert_eq!(stats.children.len(), CHILDREN_SHOWN);
    assert_eq!(stats.children[0].name, "d29", "the biggest is kept");
    assert_eq!(
        stats.directories, 30,
        "but every directory is still counted"
    );
}

// ─── safety ─────────────────────────────────────────────────────────────────

/// A symlinked loop must not be followed. `a -> ..` is a directory whose
/// contents include itself, and walking it never ends.
#[cfg(unix)]
#[test]
fn a_symlink_loop_does_not_hang_the_walk() {
    let dir = tree(&[("real/file.bin", 42)]);
    std::os::unix::fs::symlink(dir.path(), dir.path().join("real/loop")).expect("symlink");

    let stats = analyse(dir.path());
    assert_eq!(
        stats.total.bytes(),
        42,
        "the link contributes nothing and is not descended into"
    );
}

/// A subdirectory that turns out to be unreadable once the walk descends into
/// it — not merely absent, as the top-level walk already covers — must
/// contribute nothing and leave the rest of the tree intact, the same
/// never-fails contract `analyse` itself gives for its own top level.
#[cfg(unix)]
#[test]
fn a_subdirectory_that_cannot_be_read_contributes_nothing_but_does_not_fail_the_walk() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tree(&[("visible/a.bin", 10)]);
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).expect("test setup");
    fs::write(locked.join("secret.bin"), vec![b'x'; 500]).expect("test setup");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).expect("test setup");
    // Mode 000 stops nobody who is root, and a CI gate often is: then there
    // is no unreadable directory to test, so there is nothing to assert.
    let privileged = fs::read_dir(&locked).is_ok();

    let stats = analyse(dir.path());

    // Restore so the `TempDir` can clean itself up.
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("cleanup");
    if privileged {
        eprintln!("skipped: this process reads mode-000 directories (root)");
        return;
    }

    assert_eq!(
        child(&stats, "locked").map(|c| c.size.bytes()),
        Some(0),
        "an unreadable subdirectory must contribute nothing rather than failing the walk"
    );
    assert_eq!(
        stats.total.bytes(),
        10,
        "the readable subtree is still counted in full"
    );
}

/// A symlink to a file is not counted either: its target is already counted
/// wherever it really lives, and counting both inflates the total.
#[cfg(unix)]
#[test]
fn a_symlinked_file_is_not_counted_twice() {
    let dir = tree(&[("real.bin", 500)]);
    std::os::unix::fs::symlink(dir.path().join("real.bin"), dir.path().join("alias.bin"))
        .expect("symlink");

    assert_eq!(analyse(dir.path()).total.bytes(), 500);
}

/// A complete walk says so, which is what lets the caller call the numbers
/// totals rather than lower bounds.
#[test]
fn a_small_tree_is_reported_as_completely_measured() {
    let dir = tree(&[("a.bin", 1)]);
    assert!(analyse(dir.path()).coverage.is_complete());
    assert!(!matches!(analyse(dir.path()).coverage, Coverage::Partial));
}

// ─── the labels, and the answer when the walk runs out of budget ────────────

/// Every category prints a name. A missing or duplicated one would put two
/// different kinds of file on the same row of the panel.
#[test]
fn every_content_kind_has_its_own_label() {
    use super::ContentKind;

    let kinds = [
        ContentKind::Source,
        ContentKind::Document,
        ContentKind::Image,
        ContentKind::Audio,
        ContentKind::Video,
        ContentKind::Archive,
        ContentKind::Executable,
        ContentKind::Data,
        ContentKind::Other,
    ];
    let mut labels: Vec<&str> = kinds.iter().map(|kind| kind.label()).collect();
    assert!(labels.iter().all(|label| !label.is_empty()));
    labels.sort_unstable();
    let count = labels.len();
    labels.dedup();
    assert_eq!(labels.len(), count, "two kinds share a label");
}

/// The walk stops at its budget and says so, because a number that is really a
/// lower bound must not be printed as if it were the whole answer.
#[test]
fn a_walk_that_runs_out_of_budget_reports_the_numbers_as_partial() {
    use super::{analyse_within, Coverage};

    let dir = tempfile::TempDir::new().expect("test setup");
    for index in 0..5 {
        std::fs::write(dir.path().join(format!("f{index}.txt")), b"x").expect("test setup");
    }

    let complete = analyse_within(dir.path(), 100);
    assert_eq!(complete.coverage, Coverage::Complete);
    assert!(complete.coverage.is_complete());
    assert_eq!(complete.files, 5);

    let partial = analyse_within(dir.path(), 2);
    assert_eq!(partial.coverage, Coverage::Partial);
    assert!(!partial.coverage.is_complete());
    assert!(partial.files < 5, "it stopped early");
}

/// …and the budget is shared with the subdirectories, so a deep tree cannot
/// spend more than the whole walk was given.
#[test]
fn the_budget_is_shared_with_the_subtrees() {
    use super::{analyse_within, Coverage};

    let dir = tempfile::TempDir::new().expect("test setup");
    let nested = dir.path().join("a/b/c");
    std::fs::create_dir_all(&nested).expect("test setup");
    for index in 0..5 {
        std::fs::write(nested.join(format!("f{index}.txt")), b"x").expect("test setup");
    }

    let partial = analyse_within(dir.path(), 4);
    assert_eq!(partial.coverage, Coverage::Partial);
}
