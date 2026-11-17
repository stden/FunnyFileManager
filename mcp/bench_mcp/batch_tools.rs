//! Purpose: Benchmark calls of the batch copy, move and delete tools and of
//! the symlink tools.

use super::suite::json_path;
use std::path::Path;

pub fn batch_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let batch1 = tmp.join("batch1.txt");
    let batch2 = tmp.join("batch2.txt");
    let batch_dest = tmp.join("batch_dest");
    let bmove1 = tmp.join("bmove1.txt");
    let bmove2 = tmp.join("bmove2.txt");
    let bmove_dest = tmp.join("bmove_dest");

    // ── 24. batch_copy ───────────────────────────────────────────
    bench(
        "batch_copy",
        format!(
            r#"{{"paths":["{}","{}"],"destination":"{}"}}"#,
            json_path(&batch1),
            json_path(&batch2),
            json_path(&batch_dest)
        ),
    );

    // ── 25. batch_move ───────────────────────────────────────────
    bench(
        "batch_move",
        format!(
            r#"{{"paths":["{}","{}"],"destination":"{}"}}"#,
            json_path(&bmove1),
            json_path(&bmove2),
            json_path(&bmove_dest)
        ),
    );

    batch_delete_operations(bench, tmp);
}

fn batch_delete_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let bdel1 = tmp.join("bdel1.txt");
    let bdel2 = tmp.join("bdel2.txt");
    // ── 26. batch_delete ─────────────────────────────────────────
    bench(
        "batch_delete",
        format!(
            r#"{{"paths":["{}","{}"]}}"#,
            json_path(&bdel1),
            json_path(&bdel2)
        ),
    );

    symlink_operations(bench, tmp);
}

fn symlink_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let symlink_target = tmp.join("symlink_target.txt");
    let symlink_link = tmp.join("test_symlink");
    // ── 27. create_symlink ───────────────────────────────────────
    bench(
        "create_symlink",
        format!(
            r#"{{"link":"{}","target":"{}"}}"#,
            json_path(&symlink_link),
            json_path(&symlink_target)
        ),
    );

    // ── 28. read_symlink ─────────────────────────────────────────
    bench(
        "read_symlink",
        format!(r#"{{"path":"{}"}}"#, json_path(&symlink_link)),
    );
}
