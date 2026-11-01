//! Purpose: Comment, bare URI, and character-data scanning for [`super::State`].

use super::{push, run_length, State, Token};

pub(super) fn scan_comment<'a>(
    rest: &'a str,
    state: &mut State,
    runs: &mut Vec<(&'a str, Token)>,
) -> &'a str {
    let (comment, after) = if let Some(end) = rest.find("-->") {
        *state = State::Outside;
        rest.split_at(end + 3)
    } else {
        (rest, "")
    };
    scan_comment_urls(comment, runs);
    after
}

fn scan_comment_urls<'a>(mut text: &'a str, runs: &mut Vec<(&'a str, Token)>) {
    while !text.is_empty() {
        let next_url = ["http://", "https://"]
            .into_iter()
            .filter_map(|scheme| text.find(scheme))
            .min();
        let Some(pos) = next_url else {
            push(runs, text, Token::Comment);
            break;
        };
        let (before, from_url) = text.split_at(pos);
        push(runs, before, Token::Comment);
        let url_end = from_url.find(char::is_whitespace).unwrap_or(from_url.len());
        let (candidate, _) = from_url.split_at(url_end);
        let url_len = candidate.strip_suffix("-->").map_or(url_end, str::len);
        let (url, rest) = from_url.split_at(url_len);
        push(runs, url, Token::Url);
        text = rest;
    }
}

pub(super) fn scan_text_segment<'a>(mut text: &'a str, runs: &mut Vec<(&'a str, Token)>) {
    while !text.is_empty() {
        if text.starts_with("http://") || text.starts_with("https://") {
            let url_len = run_length(text, |c| !c.is_whitespace());
            let (url, rest) = text.split_at(url_len);
            push(runs, url, Token::Uri);
            text = rest;
            continue;
        }
        let next = ["http://", "https://"]
            .into_iter()
            .filter_map(|pat| text.find(pat))
            .filter(|&pos| pos > 0)
            .min()
            .unwrap_or(text.len());
        let (plain, rest) = text.split_at(next);
        push(runs, plain, Token::Normal);
        text = rest;
    }
}
