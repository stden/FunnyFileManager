//! Purpose: Recursive walkers never leave the root through a planted link.

use super::*;
use crate::tools::PathParam;

// ═══════════════════════════════════════════════════════════════
// Recursive walkers never leave the root through a planted link
// ═══════════════════════════════════════════════════════════════

#[cfg(unix)]
#[test]
fn grep_does_not_read_through_a_link_out_of_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let _outside = plant_link_out_of_root(&root, "creds", "aws_secret_access_key = hunter2");

    let result = server
        .grep(Parameters(GrepParams {
            path: ".".to_owned(),
            pattern: "aws_secret_access_key".to_owned(),
            extension: None,
            max_results: 50,
        }))
        .expect("grep runs");
    let payload = tool_json(&result);

    assert_eq!(
        payload["count"], 0,
        "the walk must not open a link that names a file outside the root, got: {payload}"
    );
}

#[cfg(unix)]
#[test]
fn replace_does_not_read_through_a_link_out_of_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let outside = plant_link_out_of_root(&root, "creds", "aws_secret_access_key = hunter2");

    let result = server
        .replace(Parameters(ReplaceParams {
            path: ".".to_owned(),
            find: "aws_secret_access_key".to_owned(),
            replace: "REDACTED".to_owned(),
            extension: None,
            name_pattern: None,
            dry_run: false,
        }))
        .expect("replace runs");
    let payload = tool_json(&result);

    assert_eq!(payload["files_affected"], 0);
    assert_eq!(
        payload["errors"], 0,
        "a link is stepped over, not opened and then refused, got: {payload}"
    );
    assert_eq!(
        fs::read_to_string(outside.path().join("secret.txt")).expect("the outside file survives"),
        "aws_secret_access_key = hunter2",
        "nothing outside the root may be rewritten"
    );
}

#[cfg(unix)]
#[test]
fn replace_dry_run_does_not_count_matches_behind_a_link() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let _outside = plant_link_out_of_root(&root, "creds", "aws_secret_access_key = hunter2");

    let result = server
        .replace(Parameters(ReplaceParams {
            path: ".".to_owned(),
            find: "aws_secret_access_key".to_owned(),
            replace: "REDACTED".to_owned(),
            extension: None,
            name_pattern: None,
            dry_run: true,
        }))
        .expect("replace runs");
    let payload = tool_json(&result);

    assert_eq!(
        payload["total_replacements"], 0,
        "a preview that counts matches is a preview that read the file, got: {payload}"
    );
}

#[cfg(unix)]
#[test]
fn glob_does_not_report_a_link_out_of_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let _outside = plant_link_out_of_root(&root, "creds", "aws_secret_access_key = hunter2");
    write_file_at(&root.join("inside.txt"), "public");

    let result = server
        .glob(Parameters(GlobParams {
            path: ".".to_owned(),
            pattern: "*".to_owned(),
            max_results: 100,
        }))
        .expect("glob runs");
    let payload = tool_json(&result);

    assert_eq!(
        payload["count"], 1,
        "only the real entry may be listed, got: {payload}"
    );
    assert_eq!(payload["results"][0]["name"], "inside.txt");
}

#[cfg(unix)]
#[test]
fn delete_by_pattern_leaves_a_matching_link_alone() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let _outside = plant_link_out_of_root(&root, "creds.tmp", "secret");

    let result = server
        .delete_by_pattern(Parameters(DeleteByPatternParams {
            path: ".".to_owned(),
            pattern: "*.tmp".to_owned(),
            include_dirs: false,
            recursive: true,
            dry_run: false,
        }))
        .expect("delete_by_pattern runs");
    let payload = tool_json(&result);

    assert_eq!(
        payload["deleted_count"], 0,
        "a pattern matches files, not the links that happen to share their names"
    );
    assert_is_symlink(&root.join("creds.tmp"));
}

#[cfg(unix)]
#[test]
fn disk_usage_does_not_total_a_tree_outside_the_root() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let outside = TempDir::new().expect("temp dir");
    write_file_at(&outside.path().join("big.bin"), &"x".repeat(64 * 1024));
    std::os::unix::fs::symlink(outside.path(), root.join("link")).expect("symlink");

    let result = server
        .disk_usage(Parameters(PathParam {
            path: ".".to_owned(),
        }))
        .expect("disk usage runs");
    let size = tool_json(&result)["size"]
        .as_u64()
        .expect("a size is reported");

    assert!(
        size < 64 * 1024,
        "following the link would have summed a tree outside the root, got {size} bytes"
    );
}
