//! Purpose: `IntelliJ` IDEA-style completion matching and ranking (exact prefix,
//! case-insensitive prefix, `CamelHumps` / `snake_case` word boundaries, and substring).

use crate::lsp::features::Completion;

pub fn filter_and_rank(items: &[Completion], prefix: &str) -> Vec<Completion> {
    if prefix.is_empty() {
        return items.to_vec();
    }
    let mut scored: Vec<((u8, usize, usize), Completion)> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let (tier, gap) =
                score(&item.label, prefix).or_else(|| score(&item.insert_text, prefix))?;
            Some(((tier, gap, index), item.clone()))
        })
        .collect();
    scored.sort_by_key(|(rank, _)| *rank);
    scored.into_iter().map(|(_, item)| item).collect()
}

fn score(candidate: &str, prefix: &str) -> Option<(u8, usize)> {
    let stem = identifier_stem(candidate);
    if stem.is_empty() {
        return None;
    }
    let len = stem.chars().count();
    if stem.starts_with(prefix) {
        return Some((0, usize::from(stem != prefix)));
    }
    if starts_with_ignore_case(stem, prefix) {
        return Some((1, usize::from(len != prefix.chars().count())));
    }
    if let Some(skipped) = camel_humps(stem, prefix) {
        return Some((2, skipped));
    }
    if let Some(pos) = stem.find(prefix) {
        return Some((3, pos));
    }
    find_ignore_case(stem, prefix).map(|pos| (4, pos))
}

fn identifier_stem(candidate: &str) -> &str {
    let trimmed = candidate.trim_start();
    let end = trimmed
        .find(|ch: char| !super::is_identifier_char(ch))
        .unwrap_or(trimmed.len());
    (end > 0)
        .then(|| trimmed.get(..end))
        .flatten()
        .unwrap_or(trimmed)
}

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    text.chars().count() >= prefix.chars().count()
        && text
            .chars()
            .zip(prefix.chars())
            .all(|(a, b)| chars_eq(a, b))
}

fn find_ignore_case(text: &str, needle: &str) -> Option<usize> {
    let text_lower: String = text.chars().flat_map(char::to_lowercase).collect();
    let needle_lower: String = needle.chars().flat_map(char::to_lowercase).collect();
    text_lower.find(&needle_lower)
}

fn camel_humps(stem: &str, prefix: &str) -> Option<usize> {
    let words = split_words(stem);
    let query: Vec<char> = prefix.chars().filter(|&ch| ch != '_').collect();
    if words.len() < 2 || query.len() < 2 {
        return None;
    }
    (0..words.len())
        .filter_map(|start| {
            let tail = words.get(start..)?;
            match_humps(tail, &query, false).map(|skipped| start + skipped)
        })
        .min()
}

fn split_words(stem: &str) -> Vec<Vec<char>> {
    let mut words: Vec<Vec<char>> = Vec::new();
    let mut current: Vec<char> = Vec::new();
    for ch in stem.chars() {
        if ch == '_' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        let boundary = ch.is_uppercase()
            && current
                .last()
                .is_some_and(|prev| prev.is_lowercase() || prev.is_ascii_digit());
        if boundary && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        current.push(ch);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn match_humps(words: &[Vec<char>], query: &[char], multi_word: bool) -> Option<usize> {
    let first = words.first()?;
    if !chars_eq(*first.first()?, *query.first()?) {
        return None;
    }
    let max_take = first
        .iter()
        .zip(query.iter())
        .take_while(|(&a, &b)| chars_eq(a, b))
        .count();
    (1..=max_take).rev().find_map(|take| {
        if take == query.len() {
            return multi_word.then_some(0);
        }
        let rem_query = query.get(take..)?;
        (0..words.len().saturating_sub(1)).find_map(|skip| {
            match_humps(words.get(1 + skip..)?, rem_query, true).map(|rest| skip + rest)
        })
    })
}

fn chars_eq(a: char, b: char) -> bool {
    a.eq_ignore_ascii_case(&b) || a.to_lowercase().eq(b.to_lowercase())
}

impl super::Editor {
    pub(super) fn open_completion_popup(
        &mut self,
        query: crate::lsp::types::Query,
        items: Vec<Completion>,
        trigger: super::CompletionTrigger,
    ) {
        self.lsp.completions = Some(Box::new(super::CompletionPool(items.clone())));
        let filtered = filter_and_rank(&items, &self.completion_prefix());
        if filtered.is_empty() && trigger == super::CompletionTrigger::AutoWhileTyping {
            self.lsp.popup = None;
            return;
        }
        let shown = if filtered.is_empty() { items } else { filtered };
        self.lsp.popup = Some(super::Dialog::Completion {
            query,
            items: shown,
            selected: 0,
        });
    }

    pub(crate) fn update_live_completion(
        &mut self,
        old_query: crate::lsp::types::Query,
        fallback: Vec<Completion>,
    ) {
        let Some(position) = self.lsp_position() else {
            return;
        };
        let prefix = self.completion_prefix();
        let pool = self
            .lsp
            .completions
            .get_or_insert_with(|| Box::new(super::CompletionPool(fallback)));
        for edit in pool
            .0
            .iter_mut()
            .filter_map(|item| item.edit.as_mut())
            .filter(|edit| edit.range.end == old_query.position)
        {
            edit.range.end = position;
        }
        let items = filter_and_rank(&pool.0, &prefix);
        self.lsp.popup = (!items.is_empty()).then(|| super::Dialog::Completion {
            query: crate::lsp::types::Query {
                stamp: self.lsp_stamp(),
                position,
                feature: old_query.feature,
            },
            items,
            selected: 0,
        });
        if self.lsp.popup.is_none() {
            self.lsp.completions = None;
        }
    }

    pub(crate) fn maybe_trigger_completion(&mut self, ch: char) {
        if self.lsp.state == super::ServerState::Ready {
            let before: String = self.line().chars().take(self.cursor_col).collect();
            self.lsp.auto_complete = super::is_identifier_char(ch)
                || ch == '.'
                || before.ends_with("::")
                || before.ends_with("->");
        }
    }
}
