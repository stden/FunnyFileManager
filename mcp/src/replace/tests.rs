use super::*;
use crate::tests::tool_json;

#[cfg(unix)]
mod permissions;

fn options(dry_run: bool) -> ReplaceOpts<'static> {
    ReplaceOpts {
        find: "x",
        replace_with: "y",
        ext_filter: None,
        name_pattern: None,
        dry_run,
    }
}

#[test]
fn expansion_limits_report_the_failure_without_modifying_the_file() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("text");
    fs::write(&file, "x".repeat(11)).unwrap();
    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, false);
    let params = ReplaceParams {
        path: ".".into(),
        find: "x".into(),
        replace: "y".repeat(1024 * 1024),
        extension: None,
        name_pattern: None,
        dry_run: false,
    };
    let result = tool_json(&replace(&server, &params).unwrap());
    drop(server);
    assert_eq!(result["errors"], 1);
    assert_eq!(result["files_affected"], 0);
    assert!(result["error_details"][0]["error"]
        .as_str()
        .unwrap()
        .contains("maximum file size"));
    assert_eq!(fs::read_to_string(file).unwrap(), "x".repeat(11));
}

#[test]
fn unreadable_text_and_oversized_sources_are_not_modified() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("binary"), [0xff, 0xfe]).unwrap();
    fs::write(root.path().join("no-match"), "keep").unwrap();
    let oversized = root.path().join("huge");
    fs::File::create(&oversized)
        .unwrap()
        .set_len(10 * 1024 * 1024 + 1)
        .unwrap();
    let mut run = ReplaceRun::default();
    replace_recursive(root.path(), &options(false), &mut run);
    assert_eq!(run.visited, 3);
    assert_eq!(run.changed, Vec::<serde_json::Value>::new());
    assert_eq!(run.errors, Vec::<serde_json::Value>::new());
    assert_eq!(fs::read(root.path().join("binary")).unwrap(), [0xff, 0xfe]);
    assert_eq!(fs::metadata(oversized).unwrap().len(), 10 * 1024 * 1024 + 1);
}

#[test]
fn exhausted_walk_budgets_do_not_visit_or_change_another_file() {
    let root = tempfile::tempdir().unwrap();
    for name in ["first", "second"] {
        fs::write(root.path().join(name), "xxx").unwrap();
    }
    let mut run = ReplaceRun {
        visited: crate::walk::WALK_BUDGET,
        ..ReplaceRun::default()
    };
    replace_recursive(root.path(), &options(true), &mut run);
    assert_eq!(run.changed, Vec::<serde_json::Value>::new());
    run.visited -= 1;
    replace_recursive(root.path(), &options(true), &mut run);
    assert_eq!(run.visited, crate::walk::WALK_BUDGET);
    assert_eq!(run.changed.len(), 1);
    assert_eq!(run.total, 3);
    for name in ["first", "second"] {
        assert_eq!(fs::read_to_string(root.path().join(name)).unwrap(), "xxx");
    }
}

#[cfg(unix)]
#[test]
fn sockets_and_symlinks_are_never_opened_as_replaceable_text() {
    let root = tempfile::tempdir().unwrap();
    let _socket = std::os::unix::net::UnixListener::bind(root.path().join("socket")).unwrap();
    std::os::unix::fs::symlink("absent", root.path().join("link")).unwrap();
    let mut run = ReplaceRun::default();
    replace_recursive(root.path(), &options(false), &mut run);
    assert_eq!(run.visited, 2);
    assert_eq!(run.changed, Vec::<serde_json::Value>::new());
    assert!(root.path().join("link").is_symlink());
}
