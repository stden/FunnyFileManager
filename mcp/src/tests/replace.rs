//! Purpose: `replace` refuses bad roots and patterns and honours its filters.

use super::*;

// ═══════════════════════════════════════════════════════════════
// replace
// ═══════════════════════════════════════════════════════════════

#[test]
fn replace_rejects_a_file_as_the_search_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "old");

    let error = server
        .replace(Parameters(ReplaceParams {
            path: "doc.txt".to_owned(),
            find: "old".to_owned(),
            replace: "new".to_owned(),
            extension: None,
            name_pattern: None,
            dry_run: false,
        }))
        .expect_err("replace walks a tree, not a single file");

    assert!(
        error.message.contains("not a directory"),
        "got: {}",
        error.message
    );
}

#[test]
fn replace_rejects_an_empty_find_pattern() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "keep");

    // `"keep".matches("")` counts five empty matches, so without this guard the
    // file would be rewritten with the replacement spliced between every char.
    let error = server
        .replace(Parameters(ReplaceParams {
            path: root.display().to_string(),
            find: String::new(),
            replace: "X".to_owned(),
            extension: None,
            name_pattern: None,
            dry_run: false,
        }))
        .expect_err("an empty needle matches everywhere");

    assert!(
        error.message.contains("find pattern must not be empty"),
        "got: {}",
        error.message
    );
    assert_eq!(
        fs::read_to_string(root.join("doc.txt")).expect("read"),
        "keep"
    );
}

#[test]
fn replace_extension_filter_leaves_other_extensions_alone() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("code.rs"), "old");
    write_file_at(&root.join("notes.txt"), "old");

    let result = server
        .replace(Parameters(ReplaceParams {
            path: root.display().to_string(),
            find: "old".to_owned(),
            replace: "new".to_owned(),
            extension: Some("rs".to_owned()),
            name_pattern: None,
            dry_run: false,
        }))
        .expect("replace runs");

    assert_eq!(tool_json(&result)["files_affected"], 1);
    assert_eq!(
        fs::read_to_string(root.join("code.rs")).expect("read"),
        "new"
    );
    assert_eq!(
        fs::read_to_string(root.join("notes.txt")).expect("read"),
        "old",
        "the extension filter must exclude, not merely order"
    );
}

#[test]
fn replace_name_pattern_filter_leaves_other_names_alone() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("keep_me.txt"), "old");
    write_file_at(&root.join("other.txt"), "old");

    let result = server
        .replace(Parameters(ReplaceParams {
            path: root.display().to_string(),
            find: "old".to_owned(),
            replace: "new".to_owned(),
            extension: None,
            name_pattern: Some("keep_*".to_owned()),
            dry_run: false,
        }))
        .expect("replace runs");

    assert_eq!(tool_json(&result)["files_affected"], 1);
    assert_eq!(
        fs::read_to_string(root.join("keep_me.txt")).expect("read"),
        "new"
    );
    assert_eq!(
        fs::read_to_string(root.join("other.txt")).expect("read"),
        "old",
        "the name filter must exclude, not merely order"
    );
}

#[test]
fn replace_rejects_oversized_name_pattern() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let too_long = "a".repeat(globmatch::MAX_PATTERN_LEN + 1);

    let err = server
        .replace(Parameters(ReplaceParams {
            path: root.display().to_string(),
            find: "old".to_owned(),
            replace: "new".to_owned(),
            extension: None,
            name_pattern: Some(too_long),
            dry_run: true,
        }))
        .expect_err("oversized name_pattern must be rejected");
    assert!(err.message.contains("glob pattern"));
}
