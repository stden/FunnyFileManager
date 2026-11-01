//! Purpose: Sorted prefix ranges include Unicode and the edges of the list.

use super::matching_in;

fn names() -> Vec<String> {
    [
        "Cargo",
        "cargo",
        "cargo-clippy",
        "cargo-fmt",
        "z",
        "ёж",
        "ёжик",
    ]
    .map(str::to_owned)
    .into()
}

#[test]
fn sorted_prefix_ranges_preserve_all_matches_and_order() {
    let names = names();
    for prefix in [
        "",
        "A",
        "Cargo",
        "c",
        "cargo",
        "cargo-fmt",
        "cargo-z",
        "z",
        "zz",
        "ё",
        "ёжик",
        "я",
    ] {
        let expected: Vec<String> = names
            .iter()
            .filter(|name| !prefix.is_empty() && name.starts_with(prefix))
            .cloned()
            .collect();
        assert_eq!(matching_in(&names, prefix), expected, "{prefix}");
    }
    assert_eq!(matching_in(&[], "cargo"), Vec::<String>::new());
}
