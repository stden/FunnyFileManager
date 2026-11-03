use super::*;

#[test]
fn append_comparison_hashes_propagates_missing_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("present.txt");
    let missing = dir.path().join("absent.txt");
    fs::write(&existing, b"abc").unwrap();

    let mut result = json!({});
    assert!(append_comparison_hashes(&mut result, &missing, &existing, None, true).is_err());
    assert!(append_comparison_hashes(&mut result, &missing, &existing, Some(0), true).is_err());
    assert!(append_comparison_hashes(&mut result, &existing, &missing, Some(0), true).is_err());
}
