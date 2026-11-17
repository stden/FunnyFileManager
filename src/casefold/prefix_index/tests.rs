//! Purpose: Unit tests verifying `PrefixIndex` against a linear-scan oracle
//! across empty, singleton, duplicate-name, and 150-element shuffled listings.

use super::PrefixIndex;
use crate::casefold::folded_name;

fn oracle_choose(
    entries: &[(String, usize)],
    prefix: &str,
    current: usize,
    next: bool,
) -> Option<usize> {
    let folded = folded_name(prefix);
    let matching: Vec<usize> = entries
        .iter()
        .filter(|(name, _)| folded_name(name).starts_with(&folded))
        .map(|(_, idx)| *idx)
        .collect();
    if next {
        matching
            .iter()
            .copied()
            .filter(|&idx| idx > current)
            .min()
            .or_else(|| matching.into_iter().min())
    } else {
        matching.into_iter().min()
    }
}

fn shuffled_entries() -> Vec<(String, usize)> {
    let prefixes = ["doc", "Doc", "img", "arc", "İstanbul", "z"];
    (0..150_usize)
        .map(|slot| {
            let row = (slot * 97 + 31) % 150;
            let head = prefixes[slot % prefixes.len()];
            let name = if slot % 15 == 0 {
                head.to_owned()
            } else {
                format!("{head}-{:02}", slot % 12)
            };
            (name, row)
        })
        .collect()
}

#[test]
fn empty_and_singleton_index_handle_hits_misses_and_wrap() {
    let empty = PrefixIndex::new(Vec::new());
    assert_eq!(empty.choose("a", 0, false), None);
    assert_eq!(empty.choose("a", 0, true), None);
    assert_eq!(
        empty.exact_indices("a").collect::<Vec<_>>(),
        Vec::<usize>::new()
    );

    let single = PrefixIndex::new(vec![(folded_name("Alpha"), 7)]);
    assert_eq!(single.choose("al", 0, false), Some(7));
    assert_eq!(single.choose("AL", 7, true), Some(7));
    assert_eq!(single.choose("AL", 3, true), Some(7));
    assert_eq!(single.choose("b", 0, false), None);
    assert_eq!(single.exact_indices("alpha").collect::<Vec<_>>(), vec![7]);
    assert_eq!(
        single.exact_indices("beta").collect::<Vec<_>>(),
        Vec::<usize>::new()
    );
}

#[test]
fn merge_sort_tree_matches_linear_oracle_across_shuffled_indices_and_ties() {
    let raw = shuffled_entries();
    let index = PrefixIndex::new(
        raw.iter()
            .map(|(name, row)| (folded_name(name), *row))
            .collect(),
    );
    for query in [
        "d", "doc", "doc-0", "doc-03", "i", "İ", "img-1", "arc", "z", "missing",
    ] {
        assert_eq!(
            index.choose(query, 0, false),
            oracle_choose(&raw, query, 0, false)
        );
        for current in [0, 1, 17, 74, 75, 148, 149, 200] {
            assert_eq!(
                index.choose(query, current, true),
                oracle_choose(&raw, query, current, true)
            );
        }
    }
}

#[test]
fn exact_indices_returns_all_folded_matches_across_ties() {
    let raw = shuffled_entries();
    let index = PrefixIndex::new(
        raw.iter()
            .map(|(name, row)| (folded_name(name), *row))
            .collect(),
    );
    let mut exact: Vec<usize> = index.exact_indices("doc").collect();
    exact.sort_unstable();
    let mut expected: Vec<usize> = raw
        .iter()
        .filter(|(n, _)| folded_name(n) == "doc")
        .map(|(_, r)| *r)
        .collect();
    expected.sort_unstable();
    assert_eq!(exact, expected);
}
