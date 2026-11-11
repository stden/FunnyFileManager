//! Purpose: Directory listings never leak whether an out-of-root symlink target
//! is a directory.

use super::*;

#[cfg(unix)]
#[test]
fn list_and_sort_directory_do_not_expose_external_directory_targets() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let outside = TempDir::new().expect("outside dir");
    fs::create_dir(outside.path().join("secret_dir")).expect("mkdir");
    std::os::unix::fs::symlink(outside.path().join("secret_dir"), root.join("ext_link"))
        .expect("symlink");
    write_file_at(&root.join("aaa.txt"), "local");

    let path = tools::PathParam {
        path: ".".to_owned(),
    };
    let listed = tool_json(&server.list_directory(Parameters(path)).expect("list"));
    assert_eq!(listed["entries"][0]["name"], "aaa.txt");
    assert_eq!(listed["entries"][1]["type"], "file");

    let sort = SortDirectoryParams {
        path: ".".to_owned(),
        sort: "name".to_owned(),
    };
    let sorted = tool_json(&server.sort_directory(Parameters(sort)).expect("sort"));
    assert_eq!(sorted["entries"][0]["name"], "aaa.txt");
    assert_eq!(sorted["entries"][1]["type"], "file");
}
