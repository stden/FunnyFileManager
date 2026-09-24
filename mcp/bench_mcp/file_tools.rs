//! Purpose: Benchmark calls of the file tools: listing, info, viewing,
//! writing, and copy, move and delete.

use super::suite::json_path;
use std::path::Path;

pub fn file_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let tmp_str = json_path(tmp);
    let test_file_str = json_path(&tmp.join("test.txt"));
    let test_dir_str = json_path(&tmp.join("subdir"));

    // ── 1. list_directory ────────────────────────────────────────
    bench("list_directory", format!(r#"{{"path":"{tmp_str}"}}"#));

    // ── 2. sort_directory ────────────────────────────────────────
    bench(
        "sort_directory",
        format!(r#"{{"path":"{test_dir_str}","sort":"size"}}"#),
    );

    // ── 3. get_file_info ─────────────────────────────────────────
    bench("get_file_info", format!(r#"{{"path":"{test_file_str}"}}"#));

    // ── 4. view_file ─────────────────────────────────────────────
    bench("view_file", format!(r#"{{"path":"{test_file_str}"}}"#));

    write_operations(bench, tmp);
}

fn write_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    // ── 5. create_file ───────────────────────────────────────────
    let new_file = tmp.join("created.txt");
    bench(
        "create_file",
        format!(
            r#"{{"path":"{}","content":"bench content"}}"#,
            json_path(&new_file)
        ),
    );

    // ── 6. write_file ────────────────────────────────────────────
    bench(
        "write_file",
        format!(
            r#"{{"path":"{}","content":"overwritten"}}"#,
            json_path(&new_file)
        ),
    );

    // ── 7. append_file ───────────────────────────────────────────
    bench(
        "append_file",
        format!(
            r#"{{"path":"{}","content":"\nappended"}}"#,
            json_path(&new_file)
        ),
    );

    // ── 8. create_directory ──────────────────────────────────────
    let new_dir = tmp.join("bench_newdir");
    bench(
        "create_directory",
        format!(r#"{{"path":"{}"}}"#, json_path(&new_dir)),
    );

    transfer_operations(bench, tmp);
}

fn transfer_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let test_file_str = json_path(&tmp.join("test.txt"));
    let copy_dest = tmp.join("copy_dest");
    let move_src = tmp.join("move_me.txt");
    let delete_file = tmp.join("delete_me.txt");
    // ── 9. copy ──────────────────────────────────────────────────
    bench(
        "copy",
        format!(
            r#"{{"source":"{}","destination":"{}"}}"#,
            test_file_str,
            json_path(&copy_dest.join("copied.txt"))
        ),
    );

    // ── 10. move_rename ──────────────────────────────────────────
    bench(
        "move_rename",
        format!(
            r#"{{"source":"{}","destination":"{}"}}"#,
            json_path(&move_src),
            json_path(&tmp.join("moved.txt"))
        ),
    );

    // ── 11. delete ───────────────────────────────────────────────
    bench(
        "delete",
        format!(r#"{{"path":"{}"}}"#, json_path(&delete_file)),
    );
}
