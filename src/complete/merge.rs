//! Purpose: Merge completion sources in first-seen order with linear expected work.

use std::collections::HashSet;

pub(super) fn append_unique(mut pool: Vec<String>, incoming: Vec<String>) -> Vec<String> {
    let mut seen: HashSet<String> = pool.iter().cloned().collect();
    pool.extend(
        incoming
            .into_iter()
            .filter(|candidate| seen.insert(candidate.clone())),
    );
    pool
}

#[cfg(test)]
mod tests;
