//! Purpose: Name sorts compare lower-cased text, and a name that is not ASCII
//! is compared letter by letter like any other.

use super::*;

fn sorted_pids(snapshot: &Snapshot, by: SortKey) -> Vec<u32> {
    let options = ViewOptions {
        sort: Sort::new(by, SortOrder::Direct),
        ..ViewOptions::default()
    };
    pids(snapshot, &build_view(snapshot.rows(), &options))
}

#[test]
fn names_with_accents_sort_by_their_lower_case_letters() {
    let snapshot = snapshot(vec![
        process(10, "zoë"),
        process(20, "zoe"),
        process(30, "Éclair"),
        process(40, "apple"),
    ]);
    assert_eq!(sorted_pids(&snapshot, SortKey::Name), [40, 20, 10, 30]);
}
