use super::*;

#[test]
fn a_fully_occupied_name_sequence_reports_failure_without_overwriting() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("file.txt");
    std::fs::write(&original, b"original").unwrap();
    for index in 1..=9999 {
        std::fs::hard_link(&original, root.path().join(format!("file_{index}.txt"))).unwrap();
    }
    let error = unique_destination(root.path(), "file.txt").unwrap_err();
    assert!(error.contains("after 9999 attempts"));
    assert_eq!(std::fs::read(original).unwrap(), b"original");
}
