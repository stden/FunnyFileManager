//! Purpose: unit tests for the "File already exists" question: answers, "Remember choice" and the cursor.

use std::path::PathBuf;

use super::{Clash, ClashAnswer, ClashFocus, EntryFacts, ExistingFile};
use crate::copy::{CopyPrompt, ExistingFiles};

fn clash(name: &str) -> Clash {
    let facts = EntryFacts {
        size: Some(1),
        modified: None,
        is_folder: false,
    };
    Clash {
        source: PathBuf::from("src").join(name),
        dest: PathBuf::from("dst").join(name),
        incoming: facts.clone(),
        existing: facts,
    }
}

fn question(names: &[&str]) -> ExistingFile {
    let clashes = names.iter().map(|name| clash(name)).collect();
    ExistingFile::new(
        CopyPrompt::new("x", &PathBuf::from("dst")),
        Vec::new(),
        PathBuf::from("src"),
        clashes,
    )
}

#[test]
fn each_answer_moves_to_the_next_clash() {
    let mut dialog = question(&["a", "b"]);
    assert_eq!(dialog.position(), (1, 2));
    assert!(!dialog.answer(ClashAnswer::Overwrite));
    assert_eq!(
        dialog.clash().map(|it| it.source.clone()),
        Some(PathBuf::from("src").join("b"))
    );
    assert!(dialog.answer(ClashAnswer::Skip));
    assert_eq!(
        dialog.answers(),
        [
            (PathBuf::from("src").join("a"), ExistingFiles::Overwrite),
            (PathBuf::from("src").join("b"), ExistingFiles::Skip),
        ]
    );
}

#[test]
fn remember_choice_answers_every_clash_left() {
    let mut dialog = question(&["a", "b", "c"]);
    dialog.answer(ClashAnswer::Skip);
    dialog.remember = true;
    assert!(dialog.answer(ClashAnswer::Overwrite));
    let policies: Vec<_> = dialog.answers().iter().map(|(_, policy)| *policy).collect();
    assert_eq!(
        policies,
        [
            ExistingFiles::Skip,
            ExistingFiles::Overwrite,
            ExistingFiles::Overwrite
        ]
    );
}

#[test]
fn keep_both_places_beside_what_is_there() {
    assert_eq!(ClashAnswer::KeepBoth.policy(), ExistingFiles::Ask);
}

#[test]
fn the_cursor_starts_on_overwrite_and_wraps_both_ways() {
    assert_eq!(ClashFocus::default(), ClashFocus::Overwrite);
    assert_eq!(ClashFocus::Cancel.next(true), ClashFocus::Remember);
    assert_eq!(ClashFocus::Remember.next(false), ClashFocus::Cancel);
    assert_eq!(ClashFocus::Overwrite.next(true), ClashFocus::Skip);
}

#[test]
fn facts_tell_a_file_from_a_folder() {
    let temp = tempfile::tempdir().expect("temp dir");
    let file = temp.path().join("f.txt");
    std::fs::write(&file, b"hello").expect("write");
    let facts = EntryFacts::of(&file);
    assert_eq!(facts.size, Some(5));
    assert!(!facts.is_folder);
    assert!(facts.modified.is_some());
    let folder = EntryFacts::of(temp.path());
    assert!(folder.is_folder);
    assert_eq!(folder.size, None);
    assert_eq!(EntryFacts::of(&temp.path().join("missing")).modified, None);
}

#[test]
fn button_hotkey_names_each_button_by_its_letter() {
    let keys: Vec<char> = (0..4).map(ExistingFile::button_hotkey).collect();
    assert_eq!(keys, ['o', 's', 'k', 'c']);
}
