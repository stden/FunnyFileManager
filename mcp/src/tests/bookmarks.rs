//! Purpose: `set_bookmark` requires an existing directory, and `list_bookmarks` /
//! `delete_bookmark` confine entries to the server root.

use super::*;
use crate::tools::bookmarks::BookmarkParams;
use serde_json::json;

#[test]
fn set_bookmark_rejects_a_file_or_missing_directory() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("doc.txt"), "text");

    let file_err = server
        .set_bookmark(Parameters(BookmarkParams {
            name: "doc".to_owned(),
            path: Some("doc.txt".to_owned()),
        }))
        .expect_err("a file is not a directory bookmark");
    assert!(
        file_err.message.contains("not a directory"),
        "got: {}",
        file_err.message
    );

    let missing_err = server
        .set_bookmark(Parameters(BookmarkParams {
            name: "ghost".to_owned(),
            path: Some("missing_dir".to_owned()),
        }))
        .expect_err("a missing directory cannot be bookmarked");
    assert!(
        missing_err.message.contains("not a directory"),
        "got: {}",
        missing_err.message
    );
}

#[test]
fn visible_and_deletable_bookmarks_are_confined_to_the_server_root() {
    let dir = TempDir::new().expect("temp dir");
    let outside = TempDir::new().expect("outside temp dir");
    let (server, root) = server_at(&dir);
    let inside_dir = root.join("work");
    fs::create_dir(&inside_dir).expect("mkdir");

    let mut bookmarks = serde_json::Map::new();
    bookmarks.insert("inside".to_owned(), json!(inside_dir.display().to_string()));
    bookmarks.insert(
        "outside".to_owned(),
        json!(outside.path().display().to_string()),
    );

    let visible = server.visible_bookmarks(&bookmarks);
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0]["name"], "inside");

    let err = server
        .remove_confined_bookmark(&mut bookmarks, "outside")
        .expect_err("bookmark outside root must not be deletable");
    assert!(
        err.message.contains("outside the allowed root"),
        "got: {}",
        err.message
    );
    assert!(bookmarks.contains_key("outside"));

    server
        .remove_confined_bookmark(&mut bookmarks, "inside")
        .expect("bookmark inside root is deletable");
    assert!(!bookmarks.contains_key("inside"));
}
