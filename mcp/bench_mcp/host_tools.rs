//! Purpose: Benchmark calls of the metadata and host tools: drives, disk
//! usage, unique names, checksums, size formatting, permissions, bookmarks,
//! system info and AWS profiles.

use super::suite::json_path;
use std::path::Path;

pub fn metadata_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let tmp_str = json_path(tmp);
    let test_dir_str = json_path(&tmp.join("subdir"));
    let test_file_str = json_path(&tmp.join("test.txt"));

    // ── 19. list_drives ──────────────────────────────────────────
    bench("list_drives", "{}".to_owned());

    // ── 20. disk_usage ───────────────────────────────────────────
    bench("disk_usage", format!(r#"{{"path":"{test_dir_str}"}}"#));

    // ── 21. unique_name ──────────────────────────────────────────
    bench(
        "unique_name",
        format!(r#"{{"directory":"{tmp_str}","filename":"test.txt"}}"#),
    );

    // ── 22. file_checksum ────────────────────────────────────────
    bench("file_checksum", format!(r#"{{"path":"{test_file_str}"}}"#));

    // ── 23. format_size_tool ───────────────────────────────────────
    bench("format_size_tool", r#"{"bytes":1048576}"#.to_owned());
}

pub fn system_operations(bench: &mut impl FnMut(&'static str, String), tmp: &Path) {
    let tmp_str = json_path(tmp);
    let test_file_str = json_path(&tmp.join("test.txt"));

    // ── 29. permissions ──────────────────────────────────────────
    bench("permissions", format!(r#"{{"path":"{test_file_str}"}}"#));

    // ── 30. set_bookmark ─────────────────────────────────────────
    bench(
        "set_bookmark",
        format!(r#"{{"name":"bench_test","path":"{tmp_str}"}}"#),
    );

    // ── 31. get_bookmark ─────────────────────────────────────────
    bench("get_bookmark", r#"{"name":"bench_test"}"#.to_owned());

    // ── 32. list_bookmarks ───────────────────────────────────────
    bench("list_bookmarks", "{}".to_owned());

    // ── 33. delete_bookmark ──────────────────────────────────────
    bench("delete_bookmark", r#"{"name":"bench_test"}"#.to_owned());

    // ── 34. system_info ──────────────────────────────────────────
    bench("system_info", "{}".to_owned());

    // ── 35. s3_list_profiles ─────────────────────────────────────
    bench("s3_list_profiles", "{}".to_owned());
}
