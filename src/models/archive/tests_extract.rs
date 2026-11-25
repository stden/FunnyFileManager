//! Unit tests: the Extract files prompt — what it offers, how the cursor
//! moves through it, and what it says when nothing selected is an archive.

use std::path::{Path, PathBuf};

use super::{ExtractField, ExtractPrompt};

fn prompt_with(names: &[&str]) -> ExtractPrompt {
    let archives = names
        .iter()
        .map(|name| PathBuf::from("/src").join(name))
        .collect();
    ExtractPrompt::new(archives, Path::new("/dest"))
}

#[test]
fn opens_on_the_destination_with_the_other_panel_offered() {
    let prompt = prompt_with(&["a.tar.gz"]);
    assert_eq!(prompt.destination.text(), "/dest");
    assert_eq!(prompt.field, ExtractField::Destination);
    assert!(!prompt.on_cancel());
}

#[test]
fn the_caption_names_one_archive_and_counts_several() {
    assert_eq!(
        prompt_with(&["a.tar.gz"]).caption(),
        "Extract \"a.tar.gz\" to:"
    );
    assert_eq!(
        prompt_with(&["a.tar", "b.zip", "c.tgz"]).caption(),
        "Extract 3 archives to:"
    );
}

#[test]
fn with_nothing_to_extract_the_caption_lists_the_formats() {
    let prompt = prompt_with(&[]);
    assert!(!prompt.can_extract());
    assert!(
        prompt.caption().contains("No archive selected"),
        "{}",
        prompt.caption()
    );
    assert!(prompt.caption().contains(".tar.gz"), "{}", prompt.caption());
}

#[test]
fn typing_edits_the_destination_and_backspace_removes_the_last_char() {
    let mut prompt = prompt_with(&["a.tar"]);
    prompt.push('/');
    prompt.push('x');
    assert_eq!(prompt.destination.text(), "/dest/x");
    prompt.backspace();
    prompt.backspace();
    assert_eq!(prompt.destination.text(), "/dest");
}

#[test]
fn the_cursor_walks_field_extract_cancel_and_wraps_both_ways() {
    let mut prompt = prompt_with(&["a.tar"]);
    prompt.advance(true);
    assert_eq!(prompt.field, ExtractField::Extract);
    prompt.advance(true);
    assert_eq!(prompt.field, ExtractField::Cancel);
    assert!(prompt.on_cancel());
    prompt.advance(true);
    assert_eq!(prompt.field, ExtractField::Destination);
    prompt.advance(false);
    assert_eq!(prompt.field, ExtractField::Cancel);
}

/// A dimmed button is skipped, as Far skips one that does not apply.
#[test]
fn a_disabled_extract_button_is_skipped_by_the_cursor() {
    let mut prompt = prompt_with(&[]);
    prompt.advance(true);
    assert_eq!(prompt.field, ExtractField::Cancel);
    prompt.advance(false);
    assert_eq!(prompt.field, ExtractField::Destination);
}
