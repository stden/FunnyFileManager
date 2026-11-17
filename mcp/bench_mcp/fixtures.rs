//! Purpose: Lays out the scratch tree whose files and folders the benchmarked
//! tool calls read, copy, move and delete.

use std::path::Path;

#[expect(
    clippy::unwrap_used,
    reason = "without its fixtures the benchmark has nothing to measure: abort"
)]
pub fn create_fixtures(root: &Path) {
    std::fs::create_dir_all(root).unwrap();
    for directory in [
        "subdir",
        "copy_dest",
        "delete_dir",
        "batch_dest",
        "bmove_dest",
        "replace_dir",
        "pattern_dir",
    ] {
        std::fs::create_dir_all(root.join(directory)).unwrap();
    }
    create_fixture_files(root).unwrap();
}

fn create_fixture_files(root: &Path) -> std::io::Result<()> {
    for (name, content) in [
        ("test.txt", "Hello, World!\nLine 2\nLine 3\nfoo bar baz\n"),
        ("subdir/a.txt", "content a"),
        ("subdir/b.rs", "fn main() {}"),
        ("subdir/c.tmp", "temporary"),
        ("move_me.txt", "move me"),
        ("delete_me.txt", "delete me"),
        ("delete_dir/x.txt", "x"),
        ("batch1.txt", "batch1"),
        ("batch2.txt", "batch2"),
        ("bmove1.txt", "bmove1"),
        ("bmove2.txt", "bmove2"),
        ("bdel1.txt", "bdel1"),
        ("bdel2.txt", "bdel2"),
        ("symlink_target.txt", "target"),
        ("replace_dir/r1.txt", "old_text here and old_text there"),
        ("replace_dir/r2.txt", "no match here"),
        ("pattern_dir/keep.txt", "keep"),
        ("pattern_dir/a.tmp", "tmp1"),
        ("pattern_dir/b.tmp", "tmp2"),
    ] {
        std::fs::write(root.join(name), content)?;
    }
    Ok(())
}
