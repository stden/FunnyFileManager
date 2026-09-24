//! Purpose: unit tests for typing into the name dialog and the names it hands
//! back, one or a list.

use super::*;

#[test]
fn typing_lands_in_the_field_under_the_cursor() {
    let mut prompt = NamePrompt::make_folder();
    for ch in "docs".chars() {
        prompt.push(ch);
    }
    prompt.focus(NameField::Link);
    prompt.cycle(true);
    prompt.focus(NameField::Target);
    for ch in "/srv/docs".chars() {
        prompt.push(ch);
    }
    assert_eq!(prompt.name(), "docs");
    assert_eq!(prompt.link_target(), "/srv/docs");

    prompt.backspace();
    assert_eq!(prompt.link_target(), "/srv/doc");
    assert_eq!(prompt.name(), "docs", "the other field is untouched");
}

/// Typing has nowhere to go while the cursor is on a button or a checkbox, and
/// must not silently land in whichever field was edited last.
#[test]
fn typing_on_a_button_changes_nothing() {
    let mut prompt = NamePrompt::make_folder();
    prompt.push('a');
    prompt.focus(NameField::Ok);
    prompt.push('b');
    prompt.backspace();
    assert_eq!(prompt.name(), "a");
}

#[test]
fn the_checkbox_answers_the_space_bar_and_the_text_fields_do_not() {
    let mut prompt = NamePrompt::make_folder();
    assert!(prompt.field().is_text(), "a space is a space in the name");

    prompt.focus(NameField::Multiple);
    assert!(!prompt.field().is_text());
    prompt.toggle();
    assert!(prompt.multiple());
    prompt.toggle();
    assert!(!prompt.multiple());
}

/// The checkbox is what turns the name field into a list, so a `;` typed
/// without it names one folder with a semicolon in it.
#[test]
fn multiple_names_splits_the_field_and_the_checkbox_is_the_switch() {
    let mut prompt = NamePrompt::make_folder();
    for ch in "one;two;three".chars() {
        prompt.push(ch);
    }
    assert_eq!(prompt.names(), vec!["one;two;three"]);

    prompt.focus(NameField::Multiple);
    prompt.toggle();
    assert_eq!(prompt.names(), vec!["one", "two", "three"]);
}

/// A list is normally typed with a trailing separator, and the blank that
/// leaves is not an error.
#[test]
fn blank_parts_of_a_list_are_dropped_rather_than_failing_it() {
    let mut prompt = NamePrompt::make_folder();
    prompt.focus(NameField::Multiple);
    prompt.toggle();
    prompt.focus(NameField::Name);
    for ch in " one ; ; two; ".chars() {
        prompt.push(ch);
    }
    assert_eq!(prompt.names(), vec!["one", "two"]);
}

/// A separator would put the folder somewhere else, which is what F7 is not
/// for — and the rule has to survive the list, or one bad part smuggles it in.
#[test]
fn a_name_carrying_a_path_separator_is_refused_in_a_list_too() {
    let mut prompt = NamePrompt::make_folder();
    prompt.focus(NameField::Multiple);
    prompt.toggle();
    prompt.focus(NameField::Name);
    for ch in "good;../escape;also/bad".chars() {
        prompt.push(ch);
    }
    assert_eq!(prompt.names(), vec!["good"]);
}

#[test]
fn a_rename_to_the_same_name_is_nothing_to_do() {
    let mut prompt = NamePrompt::rename(FileName::new("notes.txt"));
    assert_eq!(prompt.target(), None, "unchanged");
    prompt.push('!');
    assert_eq!(prompt.target(), Some("notes.txt!"));
}
