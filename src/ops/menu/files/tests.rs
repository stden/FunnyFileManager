use super::*;

#[test]
fn quote_contains_the_file_name() {
    let quoted = quote(Path::new("file"));
    assert!(quoted.contains("file"), "{quoted}");
}

#[test]
fn quote_preserves_percent_signs_without_doubling() {
    let quoted = quote(Path::new("100% done.txt"));
    assert!(quoted.contains("100% done.txt"), "{quoted}");
    assert!(!quoted.contains("%%"), "{quoted}");
}
