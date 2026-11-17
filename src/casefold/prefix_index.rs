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
        let mut ranges = vec![(0, 0); len.saturating_mul(2)];
        for (idx, (key, row)) in rows.into_iter().enumerate() {
            keys.push(key);
            pool.push(row);
            if let Some(slot) = ranges.get_mut(len + idx) {
                *slot = (idx, idx + 1);
            }
        }
        for node in (1..len).rev() {
            let left = ranges.get(node * 2).copied().unwrap_or((0, 0));
            let right = ranges.get(node * 2 + 1).copied().unwrap_or((0, 0));
            let merged = merge_children(&mut pool, left, right);
            if let Some(slot) = ranges.get_mut(node) {
                *slot = merged;
            }
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
    extend_slice(pool, li, le);
    extend_slice(pool, ri, re);
    (out_start, pool.len())
}

fn extend_slice(pool: &mut Vec<usize>, start: usize, end: usize) {
    for idx in start..end {
        if let Some(&val) = pool.get(idx) {
            pool.push(val);
        }
    }
}

fn min_option(left: Option<usize>, right: Option<usize>) -> Option<usize> {
    match (left, right) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(v), None) | (None, Some(v)) => Some(v),
        (None, None) => None,
    }
}
