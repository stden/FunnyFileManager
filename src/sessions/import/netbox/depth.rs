//! Purpose: How deep a document's elements nest, measured without parsing
//! it: `roxmltree` recurses once a level, so a file nested deep enough
//! overflows the stack before its node limit stops it.
//!
//! The scan follows the parser's grammar just far enough never to count
//! fewer levels than it recurses into: in text a `<` always opens markup; a
//! comment, CDATA section or processing instruction ends at its first
//! terminator, as the parser's does; an attribute value ends at its quote,
//! or at a `<`, where the parser gives up anyway. Counting more levels than
//! the parser reaches can only refuse a file that would not parse.

/// Markup skipped whole, by how it starts and ends.
const SKIPPED: [(&[u8], &[u8]); 3] = [(b"<!--", b"-->"), (b"<![CDATA[", b"]]>"), (b"<?", b"?>")];

/// What one piece of markup does to the nesting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Markup {
    /// `<a …>`: one level deeper until its end tag.
    Start,
    /// `<a …/>`: one level deeper for itself alone.
    Empty,
    /// `</a>`.
    End,
    /// A comment, CDATA, a processing instruction or a declaration.
    Other,
}

/// Whether an element of `text` sits more than `limit` levels deep.
pub(super) fn nested_deeper_than(text: &str, limit: usize) -> bool {
    let bytes = text.as_bytes();
    let mut depth = 0_usize;
    let mut at = 0;
    while let Some(open) = find(bytes, at, b"<") {
        let (markup, resume) = markup_at(bytes, open);
        match markup {
            Markup::Start | Markup::Empty if depth >= limit => return true,
            Markup::Start => depth += 1,
            Markup::End => depth = depth.saturating_sub(1),
            Markup::Empty | Markup::Other => {}
        }
        at = resume;
    }
    false
}

/// The markup whose `<` is at `open`, and where the scan goes on after it.
fn markup_at(bytes: &[u8], open: usize) -> (Markup, usize) {
    let rest = bytes.get(open..).unwrap_or_default();
    if let Some((start, end)) = SKIPPED.iter().find(|(start, _)| rest.starts_with(start)) {
        let resume = find(bytes, open + start.len(), end).map_or(bytes.len(), |at| at + end.len());
        return (Markup::Other, resume);
    }
    if rest.starts_with(b"</") {
        return (Markup::End, open + 2);
    }
    if rest.starts_with(b"<!") {
        return (Markup::Other, open + 2);
    }
    start_tag(bytes, open + 1)
}

/// A start tag whose name begins at `from`: empty when it ends in `/>`. A
/// tag cut short by a `<` or the end of the text counts as a start tag.
fn start_tag(bytes: &[u8], from: usize) -> (Markup, usize) {
    let mut at = from;
    while let Some(&byte) = bytes.get(at) {
        match byte {
            b'"' | b'\'' => at = value_end(bytes, at + 1, byte),
            b'<' => return (Markup::Start, at),
            b'>' if at > from && bytes.get(at - 1) == Some(&b'/') => {
                return (Markup::Empty, at + 1)
            }
            b'>' => return (Markup::Start, at + 1),
            _ => at += 1,
        }
    }
    (Markup::Start, at)
}

/// Just past the attribute value that starts at `from` and is closed by
/// `quote`; at a `<` inside it, that `<`, since the parser stops there.
fn value_end(bytes: &[u8], from: usize, quote: u8) -> usize {
    let tail = bytes.get(from..).unwrap_or_default();
    match tail.iter().position(|&byte| byte == quote || byte == b'<') {
        Some(offset) if tail.get(offset) == Some(&quote) => from + offset + 1,
        Some(offset) => from + offset,
        None => bytes.len(),
    }
}

/// Where `needle` first occurs in `bytes` at or after `from`.
fn find(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset)
}

#[cfg(test)]
mod tests;
