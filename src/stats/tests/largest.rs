//! Purpose: The largest files and subdirectories — named, ordered and capped.

use super::*;

#[test]
fn the_largest_files_are_named_with_their_path() {
    let dir = tree(&[("small.bin", 1), ("deep/huge.bin", 9999)]);
    let stats = analyse(dir.path());

    let biggest = stats.largest.first().expect("a largest file");
    assert_eq!(biggest.size.bytes(), 9999);
    assert!(
        biggest.path.contains("huge.bin"),
        "the path must lead to it: {}",
        biggest.path
    );
    assert!(biggest.path.contains("deep"), "including its directory");
}

#[test]
fn the_largest_list_is_capped_and_ordered() {
    let files: Vec<(String, u64)> = (0u64..40)
        .map(|i| (format!("f{i:02}.bin"), i * 10 + 1))
        .collect();
    let borrowed: Vec<(&str, u64)> = files.iter().map(|(n, s)| (n.as_str(), *s)).collect();
    let dir = tree(&borrowed);
    let stats = analyse(dir.path());

    assert_eq!(stats.largest.len(), LARGEST_SHOWN);
    assert!(
        stats.largest.windows(2).all(|w| w[0].size >= w[1].size),
        "largest first: {:?}",
        stats.largest
    );
    assert_eq!(
        stats.largest[0].size.bytes(),
        391,
        "and it really is the biggest"
    );
}

/// Only so many subdirectories are shown, and they are the biggest ones.
#[test]
fn the_subdirectory_list_is_capped() {
    let files: Vec<(String, u64)> = (0u64..30)
        .map(|i| (format!("d{i:02}/x.bin"), i + 1))
        .collect();
    let borrowed: Vec<(&str, u64)> = files.iter().map(|(n, s)| (n.as_str(), *s)).collect();
    let dir = tree(&borrowed);
    let stats = analyse(dir.path());

    assert_eq!(stats.children.len(), CHILDREN_SHOWN);
    assert_eq!(stats.children[0].name, "d29", "the biggest is kept");
    assert_eq!(
        stats.directories, 30,
        "but every directory is still counted"
    );
}
