//! Purpose: Prefix range index with a bottom-up merge-sort tree for O(log n)
//! first-in-listing and O(log^2 n) next-after-cursor queries.

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Default)]
pub struct PrefixIndex {
    keys: Vec<String>,
    pool: Vec<usize>,
    ranges: Vec<(usize, usize)>,
}

impl PrefixIndex {
    #[must_use]
    pub fn new(mut rows: Vec<(String, usize)>) -> Self {
        rows.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        let len = rows.len();
        let mut keys = Vec::with_capacity(len);
        let mut pool = Vec::with_capacity(len.saturating_mul(16));
        let mut ranges = vec![(0, 0); len];
        for (idx, (key, row)) in rows.into_iter().enumerate() {
            keys.push(key);
            pool.push(row);
            ranges.push((idx, idx + 1));
        }
        for node in (1..len).rev() {
            build_parent_node(&mut ranges, &mut pool, node);
        }
        Self { keys, pool, ranges }
    }

    #[must_use]
    pub fn choose(&self, prefix: &str, current: usize, next: bool) -> Option<usize> {
        let (start, end) = self.prefix_bounds(prefix)?;
        if next {
            self.range_min_at_least(start, end, current.saturating_add(1))
                .or_else(|| self.range_min_at_least(start, end, 0))
        } else {
            self.range_min_at_least(start, end, 0)
        }
    }

    pub fn exact_indices<'a>(&'a self, folded: &'a str) -> impl Iterator<Item = usize> + 'a {
        let start = self.keys.partition_point(|key| key.as_str() < folded);
        let end = self.keys.get(start..).map_or(start, |tail| {
            start + tail.partition_point(|key| key.as_str() == folded)
        });
        self.pool.get(start..end).into_iter().flatten().copied()
    }

    fn prefix_bounds(&self, prefix: &str) -> Option<(usize, usize)> {
        let folded = super::folded_name(prefix);
        let start = self
            .keys
            .partition_point(|key| key.as_str() < folded.as_str());
        let tail = self.keys.get(start..)?;
        let count = tail.partition_point(|key| key.starts_with(folded.as_str()));
        (count > 0).then_some((start, start + count))
    }

    fn range_min_at_least(&self, start: usize, end: usize, min_val: usize) -> Option<usize> {
        let len = self.keys.len();
        let mut left = len + start;
        let mut right = len + end;
        let mut best = None;
        while left < right {
            if left & 1 == 1 {
                best = min_option(best, self.node_min_at_least(left, min_val));
                left += 1;
            }
            if right & 1 == 1 {
                right -= 1;
                best = min_option(best, self.node_min_at_least(right, min_val));
            }
            left >>= 1;
            right >>= 1;
        }
        best
    }

    fn node_min_at_least(&self, node: usize, min_val: usize) -> Option<usize> {
        let &(lo, hi) = self.ranges.get(node)?;
        let slice = self.pool.get(lo..hi)?;
        if *slice.first()? >= min_val {
            return slice.first().copied();
        }
        if *slice.last()? < min_val {
            return None;
        }
        let pos = slice.partition_point(|&idx| idx < min_val);
        slice.get(pos).copied()
    }
}

#[expect(
    clippy::indexing_slicing,
    reason = "node is in 1..len and ranges has 2 * len entries"
)]
fn build_parent_node(ranges: &mut [(usize, usize)], pool: &mut Vec<usize>, node: usize) {
    ranges[node] = merge_children(pool, ranges[node * 2], ranges[node * 2 + 1]);
}

fn merge_children(
    pool: &mut Vec<usize>,
    left: (usize, usize),
    right: (usize, usize),
) -> (usize, usize) {
    let out_start = pool.len();
    let (mut li, le) = left;
    let (mut ri, re) = right;
    while li < le && ri < re {
        let lv = pool.get(li).copied().unwrap_or(0);
        let rv = pool.get(ri).copied().unwrap_or(0);
        if lv <= rv {
            pool.push(lv);
            li += 1;
        } else {
            pool.push(rv);
            ri += 1;
        }
    }
    pool.extend_from_within(li..le);
    pool.extend_from_within(ri..re);
    (out_start, pool.len())
}

fn min_option(left: Option<usize>, right: Option<usize>) -> Option<usize> {
    match (left, right) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(v), None) | (None, Some(v)) => Some(v),
        (None, None) => None,
    }
}
