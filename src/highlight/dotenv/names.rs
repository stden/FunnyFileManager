//! Purpose: The names a dotenv file declares and refers to — the `KEY=`
//! assignment prefix and `$NAME` / `${NAME}` references.

use super::{push, split_run, Token};

/// Classify only a complete assignment prefix, leaving malformed input intact.
pub(super) fn assignment<'a>(line: &'a str, runs: &mut Vec<(&'a str, Token)>) -> Option<usize> {
    let (prefix, after_equals) = line.split_once('=')?;
    let (indent, declaration) = split_run(prefix, char::is_whitespace);
    let (export_gap, key_part) = match declaration.strip_prefix("export") {
        Some(tail) if tail.starts_with(char::is_whitespace) => {
            let (gap, key_part) = split_run(tail, char::is_whitespace);
            (Some(gap), key_part)
        }
        _ => (None, declaration),
    };
    let key = key_part.trim_end();
    let padding = key_part.strip_prefix(key).unwrap_or_default();
    if !valid_name(key) {
        return None;
    }

    push(runs, indent, Token::Normal);
    if let Some(gap) = export_gap {
        runs.push(("export", Token::Keyword));
        push(runs, gap, Token::Normal);
    }
    runs.push((key, Token::Type));
    push(runs, padding, Token::Normal);
    runs.push(("=", Token::Punctuation));
    let (value_indent, value) = split_run(after_equals, char::is_whitespace);
    push(runs, value_indent, Token::Normal);
    Some(line.len() - value.len())
}

fn valid_name(name: &str) -> bool {
    name.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

pub(super) fn reference_length(text: &str) -> Option<usize> {
    let tail = text.strip_prefix('$')?;
    if let Some(braced) = tail.strip_prefix('{') {
        let (expression, _) = braced.split_once('}')?;
        let (name, _) = split_run(expression, is_name_char);
        // `$`, `{`, the expression and `}`.
        return valid_name(name).then_some(expression.len() + 3);
    }
    let (name, _) = split_run(tail, is_name_char);
    valid_name(name).then_some(name.len() + 1)
}

fn is_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}
