//! Purpose: Verify the scope of recursive repository discovery.

use super::*;

#[test]
fn current_and_nested_repositories_include_git_files_but_not_git_internals() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    for path in [
        ".git",
        "nested/one/.git",
        "nested/one/child/.git",
        ".git/hidden/.git",
        "target/build/.git",
    ] {
        fs::create_dir_all(temp.path().join(path)).expect("directory");
    }
    let worktree = temp.path().join("linked");
    fs::create_dir(&worktree).expect("worktree");
    fs::write(worktree.join(".git"), "gitdir: ../metadata").expect("Git file");
    let result = scan(temp.path(), &|| false).expect("scan");
    assert_eq!(
        result.repositories,
        [
            temp.path().to_owned(),
            worktree,
            temp.path().join("nested/one"),
            temp.path().join("nested/one/child")
        ]
    );
    assert_eq!(result.errors, 0);
}

#[test]
fn missing_root_and_cancellation_are_distinct_errors() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    assert_eq!(
        scan(&temp.path().join("missing"), &|| false)
            .err()
            .expect("missing")
            .kind(),
        io::ErrorKind::NotFound
    );
    assert_eq!(
        scan(temp.path(), &|| true).err().expect("cancelled").kind(),
        io::ErrorKind::Interrupted
    );
}

#[cfg(unix)]
#[test]
fn directory_links_cannot_escape_the_current_folder_or_create_a_cycle() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let root = temp.path().join("workspace");
    fs::create_dir_all(temp.path().join("outside/.git")).expect("outside");
    fs::create_dir(&root).expect("workspace");
    std::os::unix::fs::symlink(temp.path().join("outside"), root.join("external")).expect("link");
    std::os::unix::fs::symlink(&root, root.join("cycle")).expect("cycle");
    assert!(scan(&root, &|| false)
        .expect("scan")
        .repositories
        .is_empty());
}
