//! Purpose: Text matching and non-overlapping replacement assembly for MCP replace.

use super::ReplaceOpts;
use std::path::Path;

pub(super) fn build_replaced(
    content: &str,
    first: usize,
    projected: usize,
    find: &str,
    replace_with: &str,
) -> String {
    let mut out = String::with_capacity(projected);
    out.push_str(content.get(..first).unwrap_or_default());
    out.push_str(replace_with);
    let mut tail = content.get(first + find.len()..).unwrap_or_default();
    while let Some(pos) = tail.find(find) {
        out.push_str(tail.get(..pos).unwrap_or_default());
        out.push_str(replace_with);
        tail = tail.get(pos + find.len()..).unwrap_or_default();
    }
    out.push_str(tail);
    out
}

pub(super) fn matches_file(name: &str, opts: &ReplaceOpts<'_>) -> bool {
    let ext_ok = opts.ext_filter.is_none_or(|ext| {
        Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    });
    ext_ok && opts.name_pattern.is_none_or(|p| p.matches(name))
}
