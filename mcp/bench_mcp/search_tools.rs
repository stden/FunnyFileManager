//! Purpose: Benchmark calls of the pattern tools: search, grep, tree, glob,
//! find, replace and delete-by-pattern.

use super::suite::json_path;
use std::path::Path;

pub fn search_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let tmp_str = json_path(tmp);
    let test_dir_str = json_path(&tmp.join("subdir"));

    // ── 12. search ───────────────────────────────────────────────
    bench(
        "search",
        format!(r#"{{"path":"{tmp_str}","pattern":"txt"}}"#),
    );

    // ── 13. grep ─────────────────────────────────────────────────
    bench(
        "grep",
        format!(r#"{{"path":"{test_dir_str}","pattern":"content"}}"#),
    );

    // ── 14. tree ─────────────────────────────────────────────────
    bench("tree", format!(r#"{{"path":"{tmp_str}","depth":2}}"#));

    // ── 15. glob ─────────────────────────────────────────────────
    bench(
        "glob",
        format!(r#"{{"path":"{tmp_str}","pattern":"*.txt"}}"#),
    );

    // ── 16. find ─────────────────────────────────────────────────
    bench(
        "find",
        format!(r#"{{"path":"{tmp_str}","name":"*.txt","file_type":"file"}}"#),
    );

    replace_and_delete_operations(bench, tmp);
}

fn replace_and_delete_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let replace_dir = tmp.join("replace_dir");
    let pat_dir = tmp.join("pattern_dir");
    // ── 17. replace ──────────────────────────────────────────────
    bench(
        "replace",
        format!(
            r#"{{"path":"{}","find":"old_text","replace":"new_text"}}"#,
            json_path(&replace_dir)
        ),
    );

    // ── 18. delete_by_pattern ────────────────────────────────────
    bench(
        "delete_by_pattern",
        format!(r#"{{"path":"{}","pattern":"*.tmp"}}"#, json_path(&pat_dir)),
    );
}
