use super::*;

#[test]
fn quote_contains_the_file_name() {
    let quoted = quote(std::path::Path::new("file"));
    assert!(quoted.contains("file"), "{quoted}");
}
