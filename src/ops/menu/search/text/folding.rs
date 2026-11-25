//! Prepare only the case mappings that can affect the already-folded query.

#[derive(Clone, Copy, Debug)]
pub(super) enum Folding {
    AsciiNeedle,
    Unicode,
}

impl Folding {
    pub(super) fn for_needle(needle: &str) -> Self {
        if needle.is_ascii() {
            Self::AsciiNeedle
        } else {
            Self::Unicode
        }
    }

    pub(super) fn append(self, source: &str, out: &mut String) {
        if matches!(self, Self::AsciiNeedle) && (source.is_ascii() || !has_ascii_mapping(source)) {
            fold_ascii_into(source, out);
        } else {
            fold_into(source, out);
        }
    }
}

/// Only dotted capital I and the Kelvin sign have lowercase mappings that
/// contain ASCII. Other non-ASCII bytes cannot participate in an ASCII match.
/// Keep the full fold for these exceptions, including I's combining dot.
fn has_ascii_mapping(source: &str) -> bool {
    let bytes = source.as_bytes();
    memchr::memmem::find(bytes, "İ".as_bytes()).is_some()
        || memchr::memmem::find(bytes, "\u{212A}".as_bytes()).is_some()
}

pub(in crate::ops::menu::search) fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    fold_into(text, &mut out);
    out
}

pub(super) fn fold_into(mut source: &str, out: &mut String) {
    out.reserve(source.len());
    while !source.is_empty() {
        let ascii_len = source
            .bytes()
            .position(|b| !b.is_ascii())
            .unwrap_or(source.len());
        if let Some(ascii) = source.get(..ascii_len).filter(|s| !s.is_empty()) {
            fold_ascii_into(ascii, out);
        }
        source = source.get(ascii_len..).unwrap_or_default();
        let non_ascii_len = fold_non_ascii_prefix(source, out);
        source = source.get(non_ascii_len..).unwrap_or_default();
    }
}

fn fold_non_ascii_prefix(source: &str, out: &mut String) -> usize {
    let mut consumed = 0;
    for ch in source.chars() {
        if ch.is_ascii() {
            break;
        }
        consumed += ch.len_utf8();
        out.extend(
            ch.to_lowercase()
                .map(|letter| if letter == 'ς' { 'σ' } else { letter }),
        );
    }
    consumed
}

fn fold_ascii_into(source: &str, out: &mut String) {
    let start = out.len();
    out.push_str(source);
    out.get_mut(start..)
        .unwrap_or_default()
        .make_ascii_lowercase();
}
