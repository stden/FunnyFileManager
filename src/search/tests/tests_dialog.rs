//! Purpose: The F7 dialog — its Tab ring, the two Find buttons, and the checkboxes.

/// Backwards round the controls, and a full turn back to where it started.
/// Written against the ring rather than a list of expected pairs, so a control
/// added in the middle does not need this test rewritten.
#[test]
fn the_search_dialog_walks_backwards_and_comes_back_round() {
    use super::SearchField;

    let start = SearchField::default();
    let mut field = start;
    for _ in 0..SearchField::ORDER.len() {
        field = field.previous();
    }
    assert_eq!(field, start, "a full turn backwards");

    assert_eq!(start.next().previous(), start, "one step and back");
}

/// Only the two Find buttons commit to a direction; from anywhere else Enter
/// keeps whatever the dialog already had.
#[test]
fn only_the_find_buttons_commit_to_a_direction() {
    use super::{Direction, SearchField};

    assert_eq!(SearchField::FindNext.direction(), Some(Direction::Forward));
    assert_eq!(
        SearchField::FindPrevious.direction(),
        Some(Direction::Backward)
    );
    for field in [
        SearchField::Query,
        SearchField::Case,
        SearchField::Words,
        SearchField::Cancel,
    ] {
        assert_eq!(field.direction(), None, "{field:?} leaves it alone");
    }
}

/// Toggling `Words` a second time must return to `Anywhere` rather than stick
/// on `WholeWords` — the earlier checkbox test only flips it once.
#[test]
fn toggling_words_twice_returns_to_anywhere() {
    use super::{SearchField, SearchPrompt, WordMatch};

    let mut prompt = SearchPrompt {
        field: SearchField::Words,
        ..SearchPrompt::default()
    };
    assert_eq!(prompt.words, WordMatch::Anywhere);

    prompt.toggle();
    assert_eq!(prompt.words, WordMatch::WholeWords);

    prompt.toggle();
    assert_eq!(
        prompt.words,
        WordMatch::Anywhere,
        "toggling back must return to Anywhere"
    );
}

/// Space flips a checkbox and does nothing anywhere else, which is what keeps
/// it a space in the query field.
#[test]
fn space_flips_the_two_checkboxes_and_nothing_else() {
    use super::{Case, SearchField, SearchPrompt, WordMatch};

    let mut prompt = SearchPrompt::default();
    let (case, words) = (prompt.case, prompt.words);

    prompt.field = SearchField::Case;
    prompt.toggle();
    assert_ne!(prompt.case, case);
    prompt.toggle();
    assert_eq!(prompt.case, case, "and back again");

    prompt.field = SearchField::Words;
    prompt.toggle();
    assert_ne!(prompt.words, words);

    for field in [SearchField::Query, SearchField::Cancel] {
        let before = (prompt.case, prompt.words);
        prompt.field = field;
        prompt.toggle();
        assert_eq!((prompt.case, prompt.words), before, "{field:?}");
    }
    assert_ne!(Case::Match, Case::Ignore);
    assert_ne!(WordMatch::WholeWords, WordMatch::Anywhere);
}

#[test]
fn prompt_round_trips_search_and_classifies_fields_and_direction() {
    use crate::search::{Case, Direction, SearchField, SearchPrompt};
    assert!(!Direction::Forward.is_backward());
    assert!(Direction::Backward.is_backward());
    assert!(!SearchField::Query.is_toggle());
    assert!(SearchField::Case.is_toggle());
    assert!(SearchField::Words.is_toggle());
    assert!(SearchPrompt::from_last(None).to_search().is_none());

    let mut prompt = SearchPrompt::from_last(None);
    prompt.query.push_str("needle");
    prompt.case = Case::Match;
    prompt.direction = Direction::Backward;
    let built = prompt.to_search().expect("non-empty query");
    assert_eq!(
        SearchPrompt::from_last(Some(&built)).to_search(),
        Some(built)
    );
}
