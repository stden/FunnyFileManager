//! Purpose: the texts as the search sees them: their non-blank lines, each with
//! its 1-based number and a hash, and the comparisons that never trust a hash alone.

/// The base of the rolling hash: odd, so multiplying by it loses no bits.
pub(super) const BASE: u64 = 1_000_003;

/// A non-blank line: its text with inner whitespace collapsed, and its 1-based number.
pub(super) struct Line {
    pub(super) text: String,
    pub(super) number: usize,
    pub(super) hash: u64,
}

impl Line {
    fn new(text: String, number: usize) -> Self {
        let hash = text.bytes().fold(0_u64, |hash, byte| {
            hash.wrapping_mul(BASE).wrapping_add(u64::from(byte))
        });
        Self { text, number, hash }
    }
}

/// A line's place: the index of its text, and its index among that text's non-blank lines.
pub(super) type Place = (usize, usize);

pub(super) fn kept_lines(text: &str) -> Vec<Line> {
    text.lines()
        .enumerate()
        .filter_map(|(index, raw)| {
            let text = raw.split_whitespace().collect::<Vec<_>>().join(" ");
            (!text.is_empty()).then(|| Line::new(text, index + 1))
        })
        .collect()
}

/// The hash of the `lines` lines from `first` on.
pub(super) fn block_hash(files: &[Vec<Line>], first: Place, lines: usize) -> u64 {
    (0..lines)
        .filter_map(|offset| line_at(files, first, offset))
        .fold(0_u64, |hash, line| {
            hash.wrapping_mul(BASE).wrapping_add(line.hash)
        })
}

/// Whether the `count` lines from both places are the same, line by line.
pub(super) fn same_lines(files: &[Vec<Line>], first: Place, second: Place, count: usize) -> bool {
    (0..count).all(|offset| same_line(files, first, second, offset))
}

/// Whether the lines `offset` past both places are the same. Checks the
/// hash first, then the text, so a hash collision is never taken for a match.
pub(super) fn same_line(files: &[Vec<Line>], first: Place, second: Place, offset: usize) -> bool {
    match (
        line_at(files, first, offset),
        line_at(files, second, offset),
    ) {
        (Some(left), Some(right)) => left.hash == right.hash && left.text == right.text,
        _ => false,
    }
}

pub(super) fn line_at(files: &[Vec<Line>], (source, start): Place, offset: usize) -> Option<&Line> {
    files.get(source)?.get(start + offset)
}

pub(super) fn line_before(files: &[Vec<Line>], (source, start): Place) -> Option<&Line> {
    files.get(source)?.get(start.checked_sub(1)?)
}
