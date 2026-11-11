//! Purpose: `unique_name` finds a free name beside existing files.

use super::*;
use crate::tools::utilities::UniqueNameParams;

// ═══════════════════════════════════════════════════════════════
// unique_name
// ═══════════════════════════════════════════════════════════════

#[test]
fn unique_name_sidesteps_an_existing_file() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("report.txt"), "first");

    let result = server
        .unique_name(Parameters(UniqueNameParams {
            directory: root.display().to_string(),
            filename: "report.txt".to_owned(),
        }))
        .expect("a free name exists");
    let payload = tool_json(&result);

    assert_eq!(payload["unique_name"], "report_1.txt");
    assert_eq!(
        payload["unique_path"],
        root.join("report_1.txt").display().to_string()
    );
}

#[test]
fn unique_name_keeps_counting_past_an_occupied_suffix() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("report.txt"), "first");
    write_file_at(&root.join("report_1.txt"), "second");

    let result = server
        .unique_name(Parameters(UniqueNameParams {
            directory: root.display().to_string(),
            filename: "report.txt".to_owned(),
        }))
        .expect("a free name exists");

    assert_eq!(tool_json(&result)["unique_name"], "report_2.txt");
}

#[test]
fn unique_name_without_an_extension_just_appends_the_counter() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("README"), "first");

    let result = server
        .unique_name(Parameters(UniqueNameParams {
            directory: root.display().to_string(),
            filename: "README".to_owned(),
        }))
        .expect("a free name exists");

    assert_eq!(tool_json(&result)["unique_name"], "README_1");
}

#[test]
fn unique_name_rejects_a_file_used_as_the_directory() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    write_file_at(&root.join("notes.txt"), "notes");

    let error = server
        .unique_name(Parameters(UniqueNameParams {
            directory: "notes.txt".to_owned(),
            filename: "report.txt".to_owned(),
        }))
        .expect_err("a file is not a directory to place a name in");

    assert!(
        error.message.contains("not a directory"),
        "got: {}",
        error.message
    );
}
