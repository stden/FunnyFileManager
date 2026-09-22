//! Unit tests: the mask syntax, and the cases that make a matcher wrong.

use super::Mask;

fn hits<'a>(line: &str, names: &[&'a str]) -> Vec<&'a str> {
    let mask = Mask::parse(line);
    names
        .iter()
        .copied()
        .filter(|name| mask.matches(name))
        .collect()
}

const FILES: [&str; 7] = [
    "main.rs",
    "lib.rs",
    "notes.txt",
    "photo.JPG",
    "Makefile",
    "build.rs.tmp",
    "a.c",
];

#[test]
fn a_star_extension_takes_that_extension_and_nothing_else() {
    assert_eq!(hits("*.rs", &FILES), vec!["main.rs", "lib.rs"]);
}

#[test]
fn several_masks_are_separated_by_a_comma_or_a_semicolon() {
    assert_eq!(hits("*.rs,*.c", &FILES), vec!["main.rs", "lib.rs", "a.c"]);
    assert_eq!(hits("*.rs;*.c", &FILES), hits("*.rs,*.c", &FILES));
}

/// Far's exclusion half, which is the whole reason `|` exists: "everything,
/// except the rubbish".
#[test]
fn a_bar_excludes_and_an_empty_left_half_means_everything() {
    assert_eq!(
        hits("*|*.tmp", &FILES),
        vec![
            "main.rs",
            "lib.rs",
            "notes.txt",
            "photo.JPG",
            "Makefile",
            "a.c"
        ]
    );
    assert_eq!(hits("|*.tmp", &FILES), hits("*|*.tmp", &FILES));
}

#[test]
fn the_exclusion_wins_over_the_inclusion() {
    assert_eq!(hits("*.rs|lib.*", &FILES), vec!["main.rs"]);
}

/// A mask is typed by a person in a hurry, and `*.jpg` must find `photo.JPG`.
#[test]
fn matching_ignores_case_on_both_sides() {
    assert_eq!(hits("*.jpg", &FILES), vec!["photo.JPG"]);
    assert_eq!(hits("*.JPG", &FILES), vec!["photo.JPG"]);
    assert_eq!(hits("makefile", &FILES), vec!["Makefile"]);
}

#[test]
fn a_question_mark_is_exactly_one_character() {
    assert!(Mask::parse("?.c").matches("a.c"));
    assert!(!Mask::parse("?.c").matches("ab.c"));
    assert!(!Mask::parse("?.c").matches(".c"));
}

#[test]
fn a_name_with_no_wildcard_matches_only_itself() {
    assert!(Mask::parse("main.rs").matches("main.rs"));
    assert!(!Mask::parse("main.rs").matches("main.rs.bak"));
}

/// An empty field selects nothing. The user who cleared it did not mean "all".
#[test]
fn an_empty_mask_takes_nothing() {
    let mask = Mask::parse("   ");
    assert!(mask.is_empty());
    assert!(!mask.matches("anything"));
}

#[test]
fn a_bare_star_takes_everything_including_a_dotfile() {
    let mask = Mask::parse("*");
    assert!(mask.matches("main.rs"));
    assert!(mask.matches(".gitignore"));
    assert!(mask.matches(""));
}

/// The reason the matcher backtracks iteratively: recursively this pattern
/// takes exponential time, and a file manager must not freeze on a silly mask.
#[test]
fn a_pathological_mask_still_returns_promptly() {
    let name = "a".repeat(64);
    assert!(!Mask::parse("*a*a*a*a*a*a*b").matches(&name));
}

/// Several stars in a row are one star, and a trailing star matches the tail.
#[test]
fn runs_of_stars_behave_like_one() {
    assert!(Mask::parse("**.rs").matches("main.rs"));
    assert!(Mask::parse("main.*").matches("main.rs"));
    assert!(Mask::parse("*main*").matches("xmainy"));
}

/// Non-ASCII names go through the same lowercasing, so a Cyrillic mask works.
#[test]
fn case_folding_is_not_ascii_only() {
    assert!(Mask::parse("ОТЧЁТ*").matches("отчёт-2026.txt"));
}
