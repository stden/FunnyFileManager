//! Purpose: Tag-body and attribute scanning helpers for [`super::State`].

use super::{is_tag_name_char, push, run_length, State, Token};

pub(super) fn scan_tag_body<'a>(
    rest: &'a str,
    state: &mut State,
    runs: &mut Vec<(&'a str, Token)>,
) -> &'a str {
    let ws = run_length(rest, char::is_whitespace);
    if ws > 0 {
        let (spaces, after) = rest.split_at(ws);
        push(runs, spaces, Token::Normal);
        return after;
    }
    for close in ["/>", "?>", ">"] {
        if let Some(after) = rest.strip_prefix(close) {
            push(runs, close, Token::Punctuation);
            *state = State::Outside;
            return after;
        }
    }
    if let Some(after) = rest.strip_prefix('=') {
        push(runs, "=", Token::Tag);
        return after;
    }
    if let Some(quote) = rest.chars().next().filter(|&c| c == '"' || c == '\'') {
        return scan_quoted_attr(rest, quote, runs);
    }
    let name_len = run_length(rest, is_tag_name_char);
    if name_len > 0 {
        let (attr, after) = rest.split_at(name_len);
        emit_attribute_name(attr, runs);
        return after;
    }
    let ch_len = rest.chars().next().map_or(1, char::len_utf8);
    let (punct, after) = rest.split_at(ch_len);
    push(runs, punct, Token::Punctuation);
    after
}

fn emit_attribute_name<'a>(attr: &'a str, runs: &mut Vec<(&'a str, Token)>) {
    if let Some((prefix, local)) = attr.split_once(':') {
        let colon = attr.get(prefix.len()..=prefix.len()).unwrap_or(":");
        push(runs, prefix, Token::Punctuation);
        let colon_token = if prefix == "xmlns" {
            Token::Str
        } else {
            Token::Punctuation
        };
        push(runs, colon, colon_token);
        push(runs, local, Token::Type);
    } else if attr == "xmlns" {
        push(runs, attr, Token::Punctuation);
    } else {
        push(runs, attr, Token::Type);
    }
}

fn scan_quoted_attr<'a>(rest: &'a str, quote: char, runs: &mut Vec<(&'a str, Token)>) -> &'a str {
    let qlen = quote.len_utf8();
    let (open_q, after_open) = rest.split_at(qlen);
    let Some(inner_len) = after_open.find(quote) else {
        push(runs, rest, Token::Str);
        return "";
    };
    let (quoted, after) = rest.split_at(qlen + inner_len + qlen);
    let (inner, after_inner) = after_open.split_at(inner_len);
    let (close_q, _) = after_inner.split_at(qlen);
    if inner.contains("http://") || inner.contains("https://") {
        push(runs, open_q, Token::Punctuation);
        scan_attr_urls(inner, runs);
        push(runs, close_q, Token::Str);
    } else {
        push(runs, quoted, Token::Str);
    }
    after
}

fn scan_attr_urls<'a>(mut inner: &'a str, runs: &mut Vec<(&'a str, Token)>) {
    while !inner.is_empty() {
        let ws = run_length(inner, char::is_whitespace);
        if ws > 0 {
            let (spaces, after) = inner.split_at(ws);
            push(runs, spaces, Token::Punctuation);
            inner = after;
            continue;
        }
        let scheme = ["https://", "http://"]
            .into_iter()
            .find(|s| inner.starts_with(s));
        if let Some(scheme_str) = scheme {
            let (head, after_scheme) = inner.split_at(scheme_str.len());
            push(runs, head, Token::Punctuation);
            let url_len = run_length(after_scheme, |c| !c.is_whitespace());
            let (body, after_url) = after_scheme.split_at(url_len);
            push(runs, body, Token::Url);
            inner = after_url;
        } else {
            let chunk_len = run_length(inner, |c| !c.is_whitespace());
            let (chunk, after) = inner.split_at(chunk_len);
            push(runs, chunk, Token::Str);
            inner = after;
        }
    }
}
