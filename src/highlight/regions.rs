//! Purpose: Lays Colorer's regions over one line as token runs.

use super::{colorer, Token};

/// Lay Colorer's regions over a line, filling the gaps with plain text.
///
/// Colorer reports overlapping regions — a bracket is both `def:Symbol` and
/// `def:PairStart` — so later regions win, which is Colorer's own rule. The
/// result covers the whole line so the caller can concatenate it back.
pub(super) fn paint<'a>(line: &'a str, spans: &[colorer::Span]) -> Vec<(&'a str, Token)> {
    let width = line.chars().count();
    let mut per_char = vec![Token::Normal; width];
    for span in spans {
        for slot in per_char
            .iter_mut()
            .take(span.end.min(width))
            .skip(span.start.min(width))
        {
            *slot = span.token;
        }
    }

    // Byte offsets for every char boundary, so the runs can be sliced without
    // counting the line twice.
    let mut bounds: Vec<usize> = line.char_indices().map(|(at, _)| at).collect();
    bounds.push(line.len());

    let mut runs = Vec::new();
    let mut start = 0;
    for index in 1..=width {
        // A run ends at the end of the line, or where the token changes.
        let token = per_char.get(start).copied().unwrap_or(Token::Normal);
        let ended = index == width || per_char.get(index) != Some(&token);
        if ended {
            let (Some(from), Some(to)) = (bounds.get(start), bounds.get(index)) else {
                break;
            };
            runs.push((&line[*from..*to], token));
            start = index;
        }
    }
    runs
}
