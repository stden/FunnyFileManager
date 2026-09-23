//! In-process tests for the rules that must hold at *every* call site: the
//! registered tool list, and the promise that a symlink is never silently
//! stood in for the data it points at.
//!
//! These live beside `tests.rs` rather than inside it because they are
//! cross-cutting: each one has a twin somewhere else in `main.rs`, and keeping
//! them together is what makes a missing twin obvious.

use super::*;
use tempfile::TempDir;

/// A server sandboxed to a fresh temp directory, plus that canonical root.
fn server_at(dir: &TempDir) -> (FfmServer, PathBuf) {
    let root = dir.path().canonicalize().expect("canonicalize temp dir");
    (FfmServer::new(root.clone()), root)
}

/// The JSON payload a successful tool call carries in its text content.
fn tool_json(result: &CallToolResult) -> serde_json::Value {
    let text = result
        .content
        .first()
        .and_then(|content| content.as_text())
        .expect("a tool result carries one text content");
    serde_json::from_str(&text.text).expect("a tool payload is JSON")
}

fn write_file_at(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    fs::write(path, content).expect("write fixture file");
}

/// Assert that `path` is still a symlink.
///
/// `symlink_metadata`, never `Path::exists()`: `exists()` follows the link, so
/// it passes just as happily when the link has been replaced by a regular file
/// — or fails on a link that is merely dangling. Both make the assertion a lie.
#[cfg(unix)]
fn assert_is_symlink(path: &Path) {
    let meta = fs::symlink_metadata(path)
        .unwrap_or_else(|e| panic!("symlink_metadata {}: {e}", path.display()));
    assert!(
        meta.file_type().is_symlink(),
        "{} should still be a symlink",
        path.display()
    );
}

#[cfg(unix)]
fn assert_gone(path: &Path) {
    assert!(
        fs::symlink_metadata(path).is_err(),
        "{} should have been removed",
        path.display()
    );
}

/// A secret file in a tree the server root does not cover, plus a link to it
/// planted *inside* the root — the shape every "walk leaves the root" test
/// needs. Returns the outside directory, which must outlive the test.
#[cfg(unix)]
fn plant_link_out_of_root(root: &Path, link_name: &str, secret: &str) -> TempDir {
    let outside = TempDir::new().expect("temp dir");
    let target = outside.path().join("secret.txt");
    write_file_at(&target, secret);
    std::os::unix::fs::symlink(&target, root.join(link_name)).expect("plant the link");
    outside
}

// ═══════════════════════════════════════════════════════════════
// The registered tool list
// ═══════════════════════════════════════════════════════════════

#[test]
fn tool_names_match_the_registered_router() {
    let mut registered: Vec<String> = FfmServer::tool_router()
        .list_all()
        .into_iter()
        .map(|tool| tool.name.into_owned())
        .collect();
    registered.sort();

    let mut declared: Vec<String> = TOOL_NAMES.iter().map(|name| (*name).to_owned()).collect();
    declared.sort();

    assert_eq!(
        TOOL_NAMES.len(),
        registered.len(),
        "TOOL_NAMES is what a status screen counts — it must match the router exactly"
    );
    assert_eq!(
        declared, registered,
        "TOOL_NAMES has drifted from the tools the router registers"
    );
}

// ═══════════════════════════════════════════════════════════════
// batch_delete — a link is not the tree it points at
// ═══════════════════════════════════════════════════════════════

#[cfg(unix)]
#[test]
fn batch_delete_of_a_link_to_a_directory_keeps_the_target() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("data").join("keep.txt"), "keep");
    std::os::unix::fs::symlink(root.join("data"), root.join("link")).expect("symlink");

    let result = server
        .batch_delete(Parameters(BatchParams {
            paths: vec!["link".to_owned()],
            destination: None,
            recursive: true,
        }))
        .expect("a batch reports per-path failures instead of aborting");
    let payload = tool_json(&result);

    assert_eq!(payload["total_deleted"], 1);
    assert_eq!(payload["total_errors"], 0);
    assert_gone(&root.join("link"));
    assert_eq!(
        fs::read_to_string(root.join("data").join("keep.txt")).expect("the target tree survives"),
        "keep",
        "deleting a link must not delete what it points at"
    );
}

#[cfg(unix)]
#[test]
fn batch_delete_of_a_link_to_a_file_keeps_the_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("real.txt"), "payload");
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("alias.txt")).expect("symlink");

    let result = server
        .batch_delete(Parameters(BatchParams {
            paths: vec!["alias.txt".to_owned()],
            destination: None,
            recursive: false,
        }))
        .expect("a batch reports per-path failures instead of aborting");

    assert_eq!(tool_json(&result)["total_deleted"], 1);
    assert_gone(&root.join("alias.txt"));
    assert_eq!(
        fs::read_to_string(root.join("real.txt")).expect("the target survives"),
        "payload"
    );
}

// ═══════════════════════════════════════════════════════════════
// copy / move — link-ness is preserved, not dereferenced
// ═══════════════════════════════════════════════════════════════

#[cfg(unix)]
#[test]
fn batch_move_of_a_link_moves_the_link_not_its_target() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("real.txt"), "payload");
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("alias.txt")).expect("symlink");

    let result = server
        .batch_move(Parameters(BatchParams {
            paths: vec!["alias.txt".to_owned()],
            destination: Some("moved".to_owned()),
            recursive: true,
        }))
        .expect("a batch reports per-path failures instead of aborting");
    let payload = tool_json(&result);

    assert_eq!(
        payload["moved"][0], "alias.txt",
        "the link is what was asked for, so the link is what moves"
    );
    assert_eq!(
        fs::read_to_string(root.join("real.txt")).expect("the target stays where it was"),
        "payload"
    );
    assert_gone(&root.join("alias.txt"));
    assert_is_symlink(&root.join("moved").join("alias.txt"));
}

#[cfg(unix)]
#[test]
fn batch_copy_of_a_link_reproduces_the_link() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("real.txt"), "payload");
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("alias.txt")).expect("symlink");

    let result = server
        .batch_copy(Parameters(BatchParams {
            paths: vec!["alias.txt".to_owned()],
            destination: Some("archive".to_owned()),
            recursive: true,
        }))
        .expect("a batch reports per-path failures instead of aborting");

    assert_eq!(tool_json(&result)["copied"][0], "alias.txt");
    assert_is_symlink(&root.join("alias.txt"));
    assert_is_symlink(&root.join("archive").join("alias.txt"));
}

#[cfg(unix)]
#[test]
fn move_rename_of_a_link_moves_the_link_not_its_target() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("real.txt"), "payload");
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("alias.txt")).expect("symlink");

    server
        .move_rename(Parameters(CopyMoveParams {
            source: "alias.txt".to_owned(),
            destination: "renamed.txt".to_owned(),
        }))
        .expect("moving a link is a rename of the link");

    assert_eq!(
        fs::read_to_string(root.join("real.txt")).expect("the target stays where it was"),
        "payload"
    );
    assert_gone(&root.join("alias.txt"));
    assert_is_symlink(&root.join("renamed.txt"));
}

#[cfg(unix)]
#[test]
fn copy_of_a_link_reproduces_the_link() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("real.txt"), "payload");
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("alias.txt")).expect("symlink");

    server
        .copy(Parameters(CopyMoveParams {
            source: "alias.txt".to_owned(),
            destination: "copy.txt".to_owned(),
        }))
        .expect("copying a link recreates the link");

    assert_is_symlink(&root.join("alias.txt"));
    assert_is_symlink(&root.join("copy.txt"));
}

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

// ═══════════════════════════════════════════════════════════════
// A caller-supplied file name is one component, never a path
// ═══════════════════════════════════════════════════════════════

#[test]
fn unique_name_rejects_a_traversing_filename() {
    let dir = TempDir::new().expect("temp dir");
    let (server, _root) = server_at(&dir);

    let error = server
        .unique_name(Parameters(UniqueNameParams {
            directory: ".".to_owned(),
            filename: "../../../../etc/passwd".to_owned(),
        }))
        // Answering at all leaks whether the file is there: an existing target
        // comes back as "passwd_1", a missing one as "passwd".
        .expect_err("a file name may not walk out of the directory it is for");

    assert!(
        error.message.contains("plain file name"),
        "got: {}",
        error.message
    );
}

#[test]
fn unique_name_rejects_an_absolute_filename() {
    let dir = TempDir::new().expect("temp dir");
    let (server, _root) = server_at(&dir);

    let error = server
        .unique_name(Parameters(UniqueNameParams {
            directory: ".".to_owned(),
            // `Path::join` discards the base outright for an absolute operand.
            filename: "/etc/passwd".to_owned(),
        }))
        .expect_err("an absolute name is not a name");

    assert!(
        error.message.contains("plain file name"),
        "got: {}",
        error.message
    );
}

#[test]
fn join_child_accepts_only_one_ordinary_component() {
    let base = Path::new("/srv/root");

    assert_eq!(
        join_child(base, "report.txt").expect("a plain name is fine"),
        base.join("report.txt")
    );
    for hostile in ["../escape", "sub/dir.txt", "/etc/passwd", "..", ".", ""] {
        assert!(
            join_child(base, hostile).is_err(),
            "'{hostile}' must not be joinable onto a confined directory"
        );
    }
}

// ─── analysis tools ─────────────────────────────────────────────────────────

/// A tree with a known shape: two subdirectories, files of three kinds, and
/// one file clearly larger than the rest.
fn analysis_tree(root: &Path) {
    write_file_at(&root.join("src/main.rs"), &"x".repeat(100));
    write_file_at(&root.join("src/lib.rs"), &"x".repeat(200));
    write_file_at(&root.join("docs/guide.md"), &"x".repeat(50));
    write_file_at(&root.join("assets/logo.png"), &"x".repeat(4_000));
    write_file_at(&root.join("readme.md"), &"x".repeat(10));
}

#[test]
fn analyse_directory_reports_where_the_space_went() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    analysis_tree(&root);

    let result = server
        .analyse_directory(Parameters(AnalyseParams {
            path: root.display().to_string(),
            largest: None,
        }))
        .expect("analyse");
    let json = tool_json(&result);

    assert_eq!(json["total_size"], 4_360, "100+200+50+4000+10");
    assert_eq!(json["file_count"], 5);

    // Subdirectories, largest first.
    let subdirs = json["subdirectories"].as_array().expect("subdirectories");
    assert_eq!(subdirs.len(), 3);
    assert_eq!(subdirs[0]["name"], "assets", "the biggest comes first");
    assert_eq!(subdirs[0]["size"], 4_000);

    // Kinds, so "what is all this" is answered without opening anything.
    let kinds: Vec<&str> = json["by_kind"]
        .as_array()
        .expect("by_kind")
        .iter()
        .map(|kind| kind["kind"].as_str().unwrap_or_default())
        .collect();
    assert!(kinds.contains(&"source"), "{kinds:?}");
    assert!(kinds.contains(&"document"), "{kinds:?}");
    assert!(kinds.contains(&"image"), "{kinds:?}");

    let largest = json["largest_files"].as_array().expect("largest_files");
    assert_eq!(largest[0]["path"], "assets/logo.png");
    assert_eq!(largest[0]["size"], 4_000);
}

/// The largest-files list is as long as asked for, and no longer.
#[test]
fn analyse_directory_honours_the_largest_limit() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    analysis_tree(&root);

    let result = server
        .analyse_directory(Parameters(AnalyseParams {
            path: root.display().to_string(),
            largest: Some(2),
        }))
        .expect("analyse");
    assert_eq!(
        tool_json(&result)["largest_files"]
            .as_array()
            .expect("largest_files")
            .len(),
        2
    );
}

#[test]
fn analyse_directory_refuses_a_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("plain.txt"), "x");

    assert!(server
        .analyse_directory(Parameters(AnalyseParams {
            path: root.join("plain.txt").display().to_string(),
            largest: None,
        }))
        .is_err());
}

/// Two files with the same bytes are one group; a third with the same *size*
/// but different bytes is not part of it.
#[test]
fn find_duplicates_groups_by_content_not_by_size() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("a/report.txt"), "identical content");
    write_file_at(&root.join("b/copy.txt"), "identical content");
    // Same length, different bytes — the size check alone would call this a
    // duplicate, which is the mistake this test exists to catch.
    write_file_at(&root.join("c/other.txt"), "IDENTICAL CONTENT");
    write_file_at(&root.join("unique.txt"), "something else entirely");

    let result = server
        .find_duplicates(Parameters(DuplicatesParams {
            path: root.display().to_string(),
            min_size: None,
        }))
        .expect("find_duplicates");
    let json = tool_json(&result);

    assert_eq!(json["duplicate_groups"], 1, "{json}");
    let group = &json["groups"][0];
    assert_eq!(group["copies"], 2);
    let paths: Vec<&str> = group["paths"]
        .as_array()
        .expect("paths")
        .iter()
        .map(|path| path.as_str().unwrap_or_default())
        .collect();
    assert!(paths.iter().any(|path| path.ends_with("report.txt")));
    assert!(paths.iter().any(|path| path.ends_with("copy.txt")));

    // One of the two could be removed, so that is what is reclaimable.
    assert_eq!(json["reclaimable"], "identical content".len());
}

#[test]
fn find_duplicates_finds_nothing_in_a_tree_of_unique_files() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    for index in 0..5 {
        write_file_at(
            &root.join(format!("file-{index}.txt")),
            &format!("body {index}"),
        );
    }

    let json = tool_json(
        &server
            .find_duplicates(Parameters(DuplicatesParams {
                path: root.display().to_string(),
                min_size: None,
            }))
            .expect("find_duplicates"),
    );
    assert_eq!(json["duplicate_groups"], 0);
    assert_eq!(json["reclaimable"], 0);
}

/// Empty files are all identical to each other, which is true and useless.
#[test]
fn find_duplicates_can_ignore_small_files() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("empty-one.txt"), "");
    write_file_at(&root.join("empty-two.txt"), "");

    let with_them = tool_json(
        &server
            .find_duplicates(Parameters(DuplicatesParams {
                path: root.display().to_string(),
                min_size: Some(0),
            }))
            .expect("find_duplicates"),
    );
    assert_eq!(with_them["duplicate_groups"], 1);

    // The default floor of one byte leaves them out.
    let without = tool_json(
        &server
            .find_duplicates(Parameters(DuplicatesParams {
                path: root.display().to_string(),
                min_size: None,
            }))
            .expect("find_duplicates"),
    );
    assert_eq!(without["duplicate_groups"], 0);
}

// ─── tail ───────────────────────────────────────────────────────────────────

#[test]
fn tail_file_returns_the_last_lines() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let body: String = (1..=100).map(|n| format!("line {n}\n")).collect();
    write_file_at(&root.join("log.txt"), &body);

    let json = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: root.join("log.txt").display().to_string(),
                lines: Some(3),
            }))
            .expect("tail"),
    );
    assert_eq!(json["lines_returned"], 3);
    assert_eq!(json["content"], "line 98\nline 99\nline 100");
}

/// Asking for more lines than the file has returns the file, not an error.
#[test]
fn tail_file_asking_for_more_than_there_is_returns_all_of_it() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("short.txt"), "one\ntwo\n");

    let json = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: root.join("short.txt").display().to_string(),
                lines: Some(50),
            }))
            .expect("tail"),
    );
    assert_eq!(json["lines_returned"], 2);
    assert_eq!(json["content"], "one\ntwo");
}

/// The point of seeking from the end: a file far larger than one block still
/// returns only what was asked for.
#[test]
fn tail_file_reads_the_end_of_a_large_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    // Comfortably more than the 8 KiB block the tail reads backwards in.
    let body: String = (1..=20_000).map(|n| format!("line {n}\n")).collect();
    write_file_at(&root.join("big.log"), &body);

    let json = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: root.join("big.log").display().to_string(),
                lines: Some(2),
            }))
            .expect("tail"),
    );
    assert_eq!(json["lines_returned"], 2);
    assert_eq!(json["content"], "line 19999\nline 20000");
}

#[test]
fn tail_file_handles_an_empty_file_and_a_zero_request() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("empty.txt"), "");

    let empty = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: root.join("empty.txt").display().to_string(),
                lines: None,
            }))
            .expect("tail"),
    );
    assert_eq!(empty["lines_returned"], 0);

    write_file_at(&root.join("some.txt"), "a\nb\n");
    let none_wanted = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: root.join("some.txt").display().to_string(),
                lines: Some(0),
            }))
            .expect("tail"),
    );
    assert_eq!(none_wanted["lines_returned"], 0);
}

// ─── compare ────────────────────────────────────────────────────────────────

#[test]
fn compare_directories_names_what_differs() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("left/same.txt"), "body");
    write_file_at(&root.join("left/only-left.txt"), "x");
    write_file_at(&root.join("left/nested/grown.txt"), "short");
    write_file_at(&root.join("right/same.txt"), "body");
    write_file_at(&root.join("right/only-right.txt"), "y");
    write_file_at(&root.join("right/nested/grown.txt"), "much longer body");

    let json = tool_json(
        &server
            .compare_directories(Parameters(CompareParams {
                left: root.join("left").display().to_string(),
                right: root.join("right").display().to_string(),
            }))
            .expect("compare"),
    );

    assert_eq!(json["identical"], false);
    assert_eq!(json["only_in_left"], serde_json::json!(["only-left.txt"]));
    assert_eq!(json["only_in_right"], serde_json::json!(["only-right.txt"]));

    // The comparison reaches into subdirectories, not just the top level.
    let differing = json["differing_size"].as_array().expect("differing_size");
    assert_eq!(differing.len(), 1);
    assert!(differing[0]["path"]
        .as_str()
        .unwrap_or_default()
        .ends_with("grown.txt"));
}

#[test]
fn compare_directories_says_so_when_they_match() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    for side in ["left", "right"] {
        write_file_at(&root.join(side).join("a.txt"), "same");
        write_file_at(&root.join(side).join("deep/b.txt"), "same too");
    }

    let json = tool_json(
        &server
            .compare_directories(Parameters(CompareParams {
                left: root.join("left").display().to_string(),
                right: root.join("right").display().to_string(),
            }))
            .expect("compare"),
    );
    assert_eq!(json["identical"], true, "{json}");
}

// ─── recent ─────────────────────────────────────────────────────────────────

#[test]
fn recent_files_lists_what_was_just_written() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("fresh.txt"), "just now");
    write_file_at(&root.join("nested/also-fresh.txt"), "also now");

    let json = tool_json(
        &server
            .recent_files(Parameters(RecentParams {
                path: root.display().to_string(),
                days: Some(1),
                limit: None,
            }))
            .expect("recent"),
    );
    assert_eq!(json["count"], 2, "{json}");
}

/// A window of zero days ends now, so nothing written a moment ago is inside
/// it — the cutoff is real rather than decorative.
#[test]
fn recent_files_respects_the_window() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("fresh.txt"), "just now");

    let json = tool_json(
        &server
            .recent_files(Parameters(RecentParams {
                path: root.display().to_string(),
                days: Some(0),
                limit: None,
            }))
            .expect("recent"),
    );
    assert_eq!(json["count"], 0, "{json}");
}

#[test]
fn recent_files_honours_the_limit() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    for index in 0..10 {
        write_file_at(&root.join(format!("file-{index}.txt")), "body");
    }

    let json = tool_json(
        &server
            .recent_files(Parameters(RecentParams {
                path: root.display().to_string(),
                days: Some(1),
                limit: Some(3),
            }))
            .expect("recent"),
    );
    assert_eq!(json["count"], 3);
}

/// A symlink to a directory is not a subdirectory. `Path::is_dir()` follows
/// the link, which is how a walk would name — and, on some platforms, measure —
/// a tree the server is not allowed to see.
#[cfg(unix)]
#[test]
fn analyse_directory_does_not_treat_a_symlink_as_a_subdirectory() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("real/a.txt"), "hi");
    let outside = TempDir::new().expect("temp dir");
    write_file_at(&outside.path().join("secret.bin"), &"x".repeat(64 * 1024));
    std::os::unix::fs::symlink(outside.path(), root.join("link")).expect("symlink");

    let json = tool_json(
        &server
            .analyse_directory(Parameters(AnalyseParams {
                path: root.display().to_string(),
                largest: None,
            }))
            .expect("analyse"),
    );
    assert_only_real_subdirectories(&json);
    assert!(
        json["total_size"].as_u64().expect("total_size") < 64 * 1024,
        "following the link would have counted the secret file"
    );
}

/// `days` is a JSON integer; a caller can send `u64::MAX`. Multiplying that
/// by seconds-per-day must not panic the server.
#[test]
fn recent_files_accepts_an_enormous_window_without_panicking() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("fresh.txt"), "just now");

    let json = tool_json(
        &server
            .recent_files(Parameters(RecentParams {
                path: root.display().to_string(),
                days: Some(u64::MAX),
                limit: None,
            }))
            .expect("recent"),
    );
    assert_eq!(json["count"], 1, "{json}");
}

/// A file with no newlines is still tailed: the last (only) line, not the
/// whole file slurped as a failed line-count.
#[test]
fn tail_file_of_a_file_with_no_newlines_returns_the_whole_line() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("blob.bin"), &"x".repeat(100));

    let json = tool_json(
        &server
            .tail_file(Parameters(TailParams {
                path: root.join("blob.bin").display().to_string(),
                lines: Some(5),
            }))
            .expect("tail"),
    );
    assert_eq!(json["lines_returned"], 1);
    assert_eq!(json["content"].as_str().map(str::len), Some(100));
}

// ═══════════════════════════════════════════════════════════════════
// READING PARTS OF A FILE, COMPARING, RANKING
// ═══════════════════════════════════════════════════════════════════

#[test]
fn head_file_returns_the_first_lines() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("log.txt"), "one\ntwo\nthree\nfour\n");

    let result = server
        .head_file(Parameters(HeadParams {
            path: root.join("log.txt").display().to_string(),
            lines: Some(2),
        }))
        .expect("head_file");
    let json = tool_json(&result);
    assert_eq!(json["content"], "one\ntwo");
    assert_eq!(json["lines_returned"], 2);
    assert_eq!(json["reached_end"], false);
}

/// Asking for more than there is returns all of it, and says so.
#[test]
fn head_file_asking_for_more_than_there_is_returns_all_of_it() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("short.txt"), "only\n");

    let json = tool_json(
        &server
            .head_file(Parameters(HeadParams {
                path: root.join("short.txt").display().to_string(),
                lines: Some(100),
            }))
            .expect("head_file"),
    );
    assert_eq!(json["content"], "only");
    assert_eq!(json["reached_end"], true);
}

#[test]
fn head_file_refuses_a_directory() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    assert!(server
        .head_file(Parameters(HeadParams {
            path: root.display().to_string(),
            lines: None,
        }))
        .is_err());
}

#[test]
fn read_lines_returns_the_window_it_was_asked_for() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "a\nb\nc\nd\ne\n");

    let json = tool_json(
        &server
            .read_lines(Parameters(LineRangeParams {
                path: root.join("f.txt").display().to_string(),
                from: Some(2),
                to: Some(4),
            }))
            .expect("read_lines"),
    );
    assert_eq!(json["content"], "b\nc\nd");
    assert_eq!(json["from"], 2);
    assert_eq!(json["lines_returned"], 3);
}

/// No `to` means "to the end", and the answer says it got there.
#[test]
fn read_lines_without_an_end_runs_to_the_end() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "a\nb\nc\n");

    let json = tool_json(
        &server
            .read_lines(Parameters(LineRangeParams {
                path: root.join("f.txt").display().to_string(),
                from: Some(2),
                to: None,
            }))
            .expect("read_lines"),
    );
    assert_eq!(json["content"], "b\nc");
    assert_eq!(json["reached_end"], true);
}

/// A backwards range is refused rather than silently returning nothing.
#[test]
fn read_lines_refuses_a_backwards_range() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "a\nb\n");

    assert!(server
        .read_lines(Parameters(LineRangeParams {
            path: root.join("f.txt").display().to_string(),
            from: Some(5),
            to: Some(2),
        }))
        .is_err());
}

/// Asking for exactly every line consumes the file, and the answer must say
/// so — a paging client told `false` here asks for a page past the end.
#[test]
fn head_file_count_equal_to_total_reports_the_end() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "a\nb\nc\n");

    let json = tool_json(
        &server
            .head_file(Parameters(HeadParams {
                path: root.join("f.txt").display().to_string(),
                lines: Some(3),
            }))
            .expect("head_file"),
    );
    assert_eq!(json["lines_returned"], 3);
    assert_eq!(json["reached_end"], true);
}

/// The same boundary through `read_lines`: `to` on the file's last line.
#[test]
fn read_lines_to_the_last_line_reports_the_end() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "a\nb\nc\n");

    let json = tool_json(
        &server
            .read_lines(Parameters(LineRangeParams {
                path: root.join("f.txt").display().to_string(),
                from: Some(2),
                to: Some(3),
            }))
            .expect("read_lines"),
    );
    assert_eq!(json["content"], "b\nc");
    assert_eq!(json["reached_end"], true);
}

/// A file with no newline inside the read cap used to be slurped whole; the
/// read now gives up with a clean error instead of buffering without bound.
#[test]
fn line_readers_refuse_past_the_read_cap() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let blob = "x".repeat(usize::try_from(MAX_READ_BYTES).expect("the cap fits in usize") + 16);
    write_file_at(&root.join("blob.txt"), &blob);

    assert!(server
        .head_file(Parameters(HeadParams {
            path: root.join("blob.txt").display().to_string(),
            lines: Some(1),
        }))
        .is_err());
    assert!(server
        .read_lines(Parameters(LineRangeParams {
            path: root.join("blob.txt").display().to_string(),
            from: Some(1),
            to: None,
        }))
        .is_err());
}

#[test]
fn text_format_names_the_line_endings_and_the_final_newline() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("unix.txt"), "a\nb\n");
    write_file_at(&root.join("dos.txt"), "a\r\nb\r\n");
    write_file_at(&root.join("mixed.txt"), "a\r\nb\nc");

    let of = |name: &str| {
        tool_json(
            &server
                .text_format(Parameters(PathParam {
                    path: root.join(name).display().to_string(),
                }))
                .expect("text_format"),
        )
    };

    assert_eq!(of("unix.txt")["line_endings"], "lf");
    assert_eq!(of("unix.txt")["final_newline"], true);
    assert_eq!(of("dos.txt")["line_endings"], "crlf");
    assert_eq!(of("dos.txt")["crlf_count"], 2);
    // A CRLF must not be counted as a CR and an LF as well.
    assert_eq!(of("dos.txt")["lf_count"], 0);
    assert_eq!(of("dos.txt")["cr_count"], 0);
    assert_eq!(of("mixed.txt")["line_endings"], "mixed");
    assert_eq!(of("mixed.txt")["final_newline"], false);
}

#[test]
fn text_format_reports_a_bom_and_a_binary_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    fs::write(root.join("bom.txt"), b"\xEF\xBB\xBFhello\n").expect("write");
    fs::write(root.join("bin.dat"), b"\x00\x01\x02").expect("write");

    let of = |name: &str| {
        tool_json(
            &server
                .text_format(Parameters(PathParam {
                    path: root.join(name).display().to_string(),
                }))
                .expect("text_format"),
        )
    };
    assert_eq!(of("bom.txt")["byte_order_mark"], "utf-8");
    assert_eq!(of("bom.txt")["binary"], false);
    assert_eq!(of("bin.dat")["binary"], true);
    assert_eq!(of("bin.dat")["line_endings"], "none");
}

/// The endings sample stops at a cap, and the verdicts must not be artifacts
/// of it: a CRLF pair cut at the cap is not a lone CR, and the final newline
/// lives at the end of the file, past the sample.
#[test]
fn text_format_is_not_fooled_by_the_sample_cap() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    // One odd leading byte lands a CRLF pair astride the sample boundary.
    let mut content = String::from("x");
    content.push_str(&"\r\n".repeat(MAX_FORMAT_BYTES / 2 + 1_000));
    write_file_at(&root.join("big.txt"), &content);

    let json = tool_json(
        &server
            .text_format(Parameters(PathParam {
                path: root.join("big.txt").display().to_string(),
            }))
            .expect("text_format"),
    );
    assert_eq!(json["line_endings"], "crlf");
    assert_eq!(json["cr_count"], 0, "the cut pair must not count as a CR");
    assert_eq!(json["final_newline"], true);
}

/// The mirror case: a CR that lands on the cap boundary but is NOT followed
/// by LF is a genuine lone CR, and dropping it would hide a real "mixed"
/// verdict inside the very sample the tool claims to judge from.
#[test]
fn text_format_keeps_a_genuine_lone_cr_at_the_sample_cap() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    // LF lines plus one filler byte put a lone CR exactly on the sample's
    // last byte, and the byte just past the cap is an ordinary letter, not LF.
    let mut content = "a\n".repeat(MAX_FORMAT_BYTES / 2 - 1);
    content.push('x');
    content.push('\r');
    content.push_str(&"b\n".repeat(2_000));
    write_file_at(&root.join("big.txt"), &content);

    let json = tool_json(
        &server
            .text_format(Parameters(PathParam {
                path: root.join("big.txt").display().to_string(),
            }))
            .expect("text_format"),
    );
    assert_eq!(json["line_endings"], "mixed");
    assert_eq!(json["cr_count"], 1, "a real lone CR at the cap must count");
    assert_eq!(json["final_newline"], true);
}

#[test]
fn file_stats_counts_lines_words_and_the_longest_line() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "one two\nthree four five\n");

    let json = tool_json(
        &server
            .file_stats(Parameters(PathParam {
                path: root.join("f.txt").display().to_string(),
            }))
            .expect("file_stats"),
    );
    assert_eq!(json["lines"], 2);
    assert_eq!(json["words"], 5);
    assert_eq!(json["bytes"], 24);
    assert_eq!(json["longest_line_number"], 2);
    assert_eq!(json["longest_line_chars"], 15);
}

#[test]
fn compare_files_calls_identical_files_identical() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("a"), "same content\n");
    write_file_at(&root.join("b"), "same content\n");

    let json = tool_json(
        &server
            .compare_files(Parameters(CompareFilesParams {
                left: root.join("a").display().to_string(),
                right: root.join("b").display().to_string(),
            }))
            .expect("compare_files"),
    );
    assert_eq!(json["identical"], true);
    assert!(json["sha256"].is_string());
    assert!(json["first_difference_at"].is_null());
}

/// Where they stop agreeing is the useful half of the answer.
#[test]
fn compare_files_reports_the_first_differing_offset() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("a"), "abcdef");
    write_file_at(&root.join("b"), "abcXef");

    let json = tool_json(
        &server
            .compare_files(Parameters(CompareFilesParams {
                left: root.join("a").display().to_string(),
                right: root.join("b").display().to_string(),
            }))
            .expect("compare_files"),
    );
    assert_eq!(json["identical"], false);
    assert_eq!(json["first_difference_at"], 3);
    assert!(json["left_sha256"] != json["right_sha256"]);
}

/// Different sizes settle "identical" without hashing — but the offset is
/// still read from the bytes: these two disagree at byte 0, not at byte 5.
#[test]
fn compare_files_answers_different_sizes_without_hashing() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("a"), "short");
    write_file_at(&root.join("b"), "much longer content");

    let json = tool_json(
        &server
            .compare_files(Parameters(CompareFilesParams {
                left: root.join("a").display().to_string(),
                right: root.join("b").display().to_string(),
            }))
            .expect("compare_files"),
    );
    assert_eq!(json["identical"], false);
    assert_eq!(json["first_difference_at"], 0);
    assert!(json["left_sha256"].is_null(), "no hashing was needed");
}

/// When one file is a prefix of the other, the first difference is at the
/// shorter file's length — the first offset where only one still has a byte.
#[test]
fn compare_files_prefix_difference_is_at_the_shorter_length() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("a"), "abc");
    write_file_at(&root.join("b"), "abcdef");

    let json = tool_json(
        &server
            .compare_files(Parameters(CompareFilesParams {
                left: root.join("a").display().to_string(),
                right: root.join("b").display().to_string(),
            }))
            .expect("compare_files"),
    );
    assert_eq!(json["identical"], false);
    assert_eq!(json["first_difference_at"], 3);
}

#[test]
fn largest_files_ranks_by_size_biggest_first() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("small.txt"), "x");
    write_file_at(&root.join("nested/big.txt"), &"y".repeat(500));
    write_file_at(&root.join("medium.txt"), &"z".repeat(50));

    let json = tool_json(
        &server
            .largest_files(Parameters(LargestParams {
                path: root.display().to_string(),
                limit: Some(2),
                min_size: None,
            }))
            .expect("largest_files"),
    );
    let files = json["files"].as_array().expect("a list of files");
    assert_eq!(files.len(), 2, "the limit is honoured");
    assert_eq!(files[0]["size"], 500);
    assert!(files[0]["path"].as_str().expect("path").contains("big.txt"));
    assert_eq!(files[1]["size"], 50);
    // The total covers everything considered, not just what was reported.
    assert_eq!(json["total_bytes"], 551);
    // A complete walk must say so — the same flag analyse_directory carries,
    // so a budget-stopped partial ranking is never silent.
    assert_eq!(json["truncated"], false);
}

#[test]
fn largest_files_skips_what_is_under_the_floor() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("tiny"), "x");
    write_file_at(&root.join("big"), &"y".repeat(100));

    let json = tool_json(
        &server
            .largest_files(Parameters(LargestParams {
                path: root.display().to_string(),
                limit: None,
                min_size: Some(10),
            }))
            .expect("largest_files"),
    );
    assert_eq!(json["files"].as_array().expect("files").len(), 1);
}

#[test]
fn largest_files_refuses_a_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("f.txt"), "x");
    assert!(server
        .largest_files(Parameters(LargestParams {
            path: root.join("f.txt").display().to_string(),
            limit: None,
            min_size: None,
        }))
        .is_err());
}

/// Something every machine running these tests has on PATH.
#[test]
fn which_finds_an_executable_that_is_there() {
    let dir = TempDir::new().expect("temp dir");
    let (server, _root) = server_at(&dir);
    let name = if cfg!(windows) { "cmd" } else { "sh" };

    let json = tool_json(
        &server
            .which(Parameters(WhichParams {
                name: name.to_owned(),
            }))
            .expect("which"),
    );
    assert_eq!(json["found"], true, "{name} should be on PATH");
    assert!(json["path"].is_string());
    assert!(!json["all"].as_array().expect("all").is_empty());
}

#[test]
fn which_says_so_when_there_is_nothing_to_find() {
    let dir = TempDir::new().expect("temp dir");
    let (server, _root) = server_at(&dir);

    let json = tool_json(
        &server
            .which(Parameters(WhichParams {
                name: "ffm-no-such-executable-anywhere".to_owned(),
            }))
            .expect("which"),
    );
    assert_eq!(json["found"], false);
    assert!(json["path"].is_null());
}

/// A plain file named like the command must not shadow the real one: `which`
/// answers "what would run", and a README on PATH does not run.
#[cfg(unix)]
#[test]
fn which_skips_a_non_executable_decoy_earlier_on_path() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().expect("temp dir");
    let decoy_dir = dir.path().join("decoys");
    let real_dir = dir.path().join("bin");
    write_file_at(&decoy_dir.join("mytool"), "just a note, mode 644");
    let real = real_dir.join("mytool");
    write_file_at(&real, "#!/bin/sh\n");
    fs::set_permissions(&real, fs::Permissions::from_mode(0o755)).expect("chmod");

    let path_list = std::env::join_paths([&decoy_dir, &real_dir]).expect("join paths");
    assert_eq!(which_all("mytool", &path_list), vec![real]);
}

/// A path is not a name: `which` answers about PATH, and taking a path here
/// would quietly answer a different question.
#[test]
fn which_refuses_anything_with_a_directory_in_it() {
    let dir = TempDir::new().expect("temp dir");
    let (server, _root) = server_at(&dir);
    for name in ["", "/bin/sh", "dir\\thing"] {
        assert!(
            server
                .which(Parameters(WhichParams {
                    name: name.to_owned()
                }))
                .is_err(),
            "{name:?} must be refused"
        );
    }
}

#[cfg(unix)]
fn assert_only_real_subdirectories(json: &serde_json::Value) {
    let names: Vec<&str> = json["subdirectories"]
        .as_array()
        .expect("subdirectories")
        .iter()
        .map(|child| child["name"].as_str().unwrap_or_default())
        .collect();
    assert!(
        !names.contains(&"link"),
        "a symlink must not be listed as a subdirectory: {names:?}"
    );
    assert!(
        names.contains(&"real"),
        "a real directory is still listed: {names:?}"
    );
}
