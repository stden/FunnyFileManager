//! Unit tests for three-way merging into a file mapping lacking the merged section.

use super::*;

#[test]
fn merge_creates_missing_section_mapping_in_target_file() {
    let loaded: Mapping = serde_norway::from_str("section:\n  a: 1\n").unwrap();
    let on_disk: Mapping = serde_norway::from_str("section:\n  a: 1\n  b: 2\n").unwrap();
    let current: Mapping = serde_norway::from_str("section:\n  a: 1\n  c: 3\n").unwrap();
    let mut file = Mapping::new();

    merge(
        &mut file,
        &Sides {
            on_disk: &on_disk,
            loaded: &loaded,
            current: &current,
        },
    )
    .expect("merge into empty file mapping");

    let section = file["section"].as_mapping().expect("created mapping");
    assert_eq!(section["c"], Value::from(3));
}
