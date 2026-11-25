//! Purpose: Size-descending duplicate-content grouping with edge-sample pre-filtering.

use crate::checksum::{edge_fingerprint, EDGE_THRESHOLD_BYTES};
use crate::{checksum_of, format_size, saturating_u64, CollectedFile};
use serde_json::json;
use std::collections::BTreeMap;

pub(super) fn duplicate_groups(
    files: &[CollectedFile],
    min_size: u64,
) -> (Vec<serde_json::Value>, u64) {
    let mut by_size: BTreeMap<u64, Vec<&CollectedFile>> = BTreeMap::new();
    for file in files.iter().filter(|file| file.size >= min_size) {
        by_size.entry(file.size).or_default().push(file);
    }

    let mut groups = Vec::new();
    let mut wasted = 0u64;
    for (&size, candidates) in by_size.iter().rev().filter(|(_, group)| group.len() > 1) {
        if size > EDGE_THRESHOLD_BYTES {
            record_large_duplicates(size, candidates, &mut groups, &mut wasted);
        } else {
            record_hashed_duplicates(size, candidates, &mut groups, &mut wasted);
        }
    }
    (groups, wasted)
}

fn record_large_duplicates(
    size: u64,
    candidates: &[&CollectedFile],
    groups: &mut Vec<serde_json::Value>,
    wasted: &mut u64,
) {
    let mut by_edge: BTreeMap<[u8; 32], Vec<&CollectedFile>> = BTreeMap::new();
    for &file in candidates {
        if let Some(edge) = edge_fingerprint(&file.path, size) {
            by_edge.entry(edge).or_default().push(file);
        }
    }
    for subgroup in by_edge.into_values().filter(|sub| sub.len() > 1) {
        record_hashed_duplicates(size, &subgroup, groups, wasted);
    }
}

fn record_hashed_duplicates(
    size: u64,
    candidates: &[&CollectedFile],
    groups: &mut Vec<serde_json::Value>,
    wasted: &mut u64,
) {
    let mut by_hash: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for file in candidates {
        if let Some(hash) = checksum_of(&file.path) {
            by_hash.entry(hash).or_default().push(&file.relative);
        }
    }
    for (hash, paths) in by_hash.into_iter().filter(|(_, paths)| paths.len() > 1) {
        *wasted += size * (saturating_u64(paths.len()) - 1);
        groups.push(json!({
            "checksum": hash, "size": size, "size_human": format_size(size),
            "copies": paths.len(), "paths": paths,
        }));
    }
}
