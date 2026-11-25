//! Purpose: `analyse_directory` names where the space went, without following a
//! link.

use super::*;

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
