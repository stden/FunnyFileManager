//! Purpose: `find_duplicates` groups files by content.

use super::*;

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
