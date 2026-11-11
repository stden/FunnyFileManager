//! Steps for the panel-sorting workflow BDD scenarios.
//!
//! "the left panel contains the file {string}" is defined in
//! `quick_search.rs` and is not redefined here.
//!
//! Steps shared with other modules (workspace setup, key dispatch) are NOT
//! redefined here — they live in `common.rs`.

use std::collections::BTreeMap;
use std::path::Path;

use super::BehaviorWorld;
use crate::models::{Panel, SortMode, SortOrder};
use cucumber::{given, then};

#[given("the left panel starts with reverse name sorting")]
fn reverse_name_sort(world: &mut BehaviorWorld) {
    let case = world.case();
    crate::ops::sortmenu::set(&mut case.app, SortMode::Name);
    crate::ops::sortmenu::order(&mut case.app, SortOrder::Reversed);
    case.paint();
}

fn names(panel: &Panel) -> Vec<&str> {
    panel
        .entries
        .iter()
        .filter(|entry| entry.name != "..")
        .map(|entry| entry.name.as_str())
        .collect()
}

/// Assert that left-panel entries (excluding "..") are in non-decreasing
/// lexicographic order.
#[then("the left panel entries are sorted alphabetically")]
fn sorted_alphabetically(world: &mut BehaviorWorld) {
    assert_name_order(&world.case().app.left_panel, false);
}

#[then("the left panel entries are sorted in reverse alphabetical order")]
fn sorted_in_reverse(world: &mut BehaviorWorld) {
    assert_name_order(&world.case().app.left_panel, true);
}

fn assert_name_order(panel: &Panel, reversed: bool) {
    assert_eq!(panel.sort_mode, SortMode::Name);
    assert_eq!(panel.sort_order.is_reversed(), reversed);
    let names = names(panel);
    assert!(names.len() >= 2, "sorting must compare multiple entries");
    for pair in names.windows(2) {
        let ordered = if reversed {
            pair[0] >= pair[1]
        } else {
            pair[0] <= pair[1]
        };
        assert!(
            ordered,
            "incorrect name order: {names:?}, reversed={reversed}"
        );
    }
}

/// Verify that every extension occupies a contiguous run of indices.
fn assert_extension_contiguous(ext: &str, indices: &[usize]) {
    if let (Some(&min_i), Some(&max_i)) = (indices.iter().min(), indices.iter().max()) {
        assert_eq!(
            max_i - min_i + 1,
            indices.len(),
            "extension {ext:?} is not grouped: indices {indices:?}"
        );
    }
}

/// Assert that files with the same extension appear as a contiguous block.
#[then("files with the same extension are grouped together")]
fn grouped_by_extension(world: &mut BehaviorWorld) {
    let panel = &world.case().app.left_panel;
    assert_eq!(panel.sort_mode, SortMode::Extension);
    assert_eq!(panel.sort_order, SortOrder::Direct);
    let names = names(panel);

    let mut ext_indices: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, name) in names.iter().enumerate() {
        let ext = Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        ext_indices.entry(ext).or_default().push(i);
    }

    assert!(
        ext_indices.len() >= 2,
        "fixture must contain multiple file types"
    );
    assert!(ext_indices.values().any(|indices| indices.len() > 1));
    for (ext, indices) in &ext_indices {
        assert_extension_contiguous(ext, indices);
    }
}
