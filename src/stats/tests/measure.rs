//! Unit tests: each immediate entry of a folder is measured by its shape, links
//! are never followed, and an unlistable folder is an error rather than an empty listing.

use super::*;

fn by_name(measured: &[Measured], name: &str) -> Measured {
    measured
        .iter()
        .find(|entry| entry.name == name)
        .cloned()
        .unwrap_or_else(|| panic!("no entry named {name}: {measured:?}"))
}

#[test]
fn each_entry_is_measured_by_its_shape() {
    let dir = tree(&[("big/a.bin", 300), ("big/sub/b.bin", 200), ("note.txt", 7)]);
    let mut budget = Budget::new(100, None);
    let measured = measure_entries(dir.path(), &mut budget).expect("listed");
    assert_eq!(
        by_name(&measured, "big"),
        Measured {
            name: "big".to_owned(),
            shape: Shape::Folder,
            bytes: 500,
            unread: 0,
            git_entries: 0,
        }
    );
    assert_eq!(by_name(&measured, "note.txt").shape, Shape::File);
    assert_eq!(by_name(&measured, "note.txt").bytes, 7);
    assert_eq!(budget.stop(), None);
}

/// A `.git` folder anywhere below an entry is counted on that entry, so that
/// the entry can be kept; a file of another name is not.
#[test]
fn a_git_entry_anywhere_below_is_counted_on_its_entry() {
    let dir = tree(&[
        ("vendor/pkg/.git/HEAD", 4),
        ("vendor/pkg/src/lib.rs", 4),
        ("plain/readme.md", 4),
    ]);
    let mut budget = Budget::new(100, None);
    let measured = measure_entries(dir.path(), &mut budget).expect("listed");
    assert_eq!(by_name(&measured, "vendor").git_entries, 1);
    assert_eq!(by_name(&measured, "plain").git_entries, 0);
}

#[cfg(unix)]
#[test]
fn a_link_is_reported_as_a_link_and_measures_nothing() {
    let dir = tree(&[("real/a.bin", 50)]);
    std::os::unix::fs::symlink(dir.path().join("real"), dir.path().join("link")).expect("link");
    let mut budget = Budget::new(100, None);
    let measured = measure_entries(dir.path(), &mut budget).expect("listed");
    let link = by_name(&measured, "link");
    assert_eq!(link.shape, Shape::Link);
    assert_eq!(link.bytes, 0, "the target is not measured through the link");
    assert_eq!(by_name(&measured, "real").bytes, 50);
}

#[test]
fn the_entry_budget_stops_the_listing_and_names_it() {
    let dir = tree(&[
        ("a.bin", 1),
        ("b.bin", 1),
        ("c.bin", 1),
        ("d.bin", 1),
        ("e.bin", 1),
    ]);
    let mut budget = Budget::new(3, None);
    let measured = measure_entries(dir.path(), &mut budget).expect("listed");
    assert_eq!(measured.len(), 3);
    assert_eq!(budget.stop(), Some(Stop::Entries));
}

#[test]
fn the_time_budget_stops_the_listing_and_names_it() {
    let dir = tree(&[("a.bin", 1)]);
    let mut budget = Budget::new(100, Some(Duration::ZERO));
    let measured = measure_entries(dir.path(), &mut budget).expect("listed");
    assert_eq!(measured.len(), 0);
    assert_eq!(budget.stop(), Some(Stop::Time));
}

#[test]
fn a_folder_that_cannot_be_listed_is_an_error_not_an_empty_listing() {
    let mut budget = Budget::new(10, None);
    let result = measure_entries(Path::new("/definitely/not/here"), &mut budget);
    assert!(result.is_err(), "{result:?}");
}
