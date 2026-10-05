//! Unit tests for the F7 search: shared builders here, one topic per child
//! module.

use super::*;

fn lines(text: &[&str]) -> Vec<String> {
    text.iter().map(|line| (*line).to_owned()).collect()
}

/// A plain search: forwards, ignoring case, matching anywhere.
fn search(query: &str) -> Search {
    Search::new(Needle::new(query).expect("test queries are never empty"))
}

/// A search with one option changed from the default.
fn with(query: &str, change: impl FnOnce(&mut Search)) -> Search {
    let mut search = search(query);
    change(&mut search);
    search
}

mod tests_dialog;
mod tests_find;
mod tests_options;
