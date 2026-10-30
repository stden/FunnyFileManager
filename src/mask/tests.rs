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
    assert!(Mask::parse("ΟΔΟΣ.txt").matches("ΟΔΟΣ.txt"));
    assert!(Mask::parse("οδος.txt").matches("ΟΔΟΣ.txt"));
    assert!(Mask::parse("ΟΔΟΣ.txt").matches("οδος.txt"));
}

const SEPARATED: [&str; 4] = ["semi;colon", "comma,name", "with space.txt", "plain.txt"];

/// Far: "It is allowed to put any of the masks in quotes but not the whole
/// list" — the way to name a file whose name holds a separator.
#[test]
fn a_quoted_mask_takes_its_separators_literally() {
    assert_eq!(hits("\"semi;colon\"", &SEPARATED), vec!["semi;colon"]);
    assert_eq!(hits("\"comma,name\"", &SEPARATED), vec!["comma,name"]);
    assert_eq!(
        hits("\"with space.txt\"", &SEPARATED),
        vec!["with space.txt"]
    );
}

/// Quotes wrap one mask of a list; the others still split and still glob.
#[test]
fn quoted_and_plain_masks_mix_in_one_list() {
    assert_eq!(
        hits("\"semi;*\", plain.*", &SEPARATED),
        vec!["semi;colon", "plain.txt"]
    );
    assert_eq!(
        hits("*|\"comma,name\";*.txt", &SEPARATED),
        vec!["semi;colon"]
    );
}

/// A bar inside quotes is part of the name, not the start of the exclusions.
#[test]
fn a_quoted_bar_does_not_start_the_exclusions() {
    assert!(Mask::parse("\"a|b\"").matches("a|b"));
    assert!(!Mask::parse("\"a|b\"").matches("a"));
}

/// Blanks inside the quotes are the name's own; the ones around them are not.
#[test]
fn only_the_blanks_outside_the_quotes_are_trimmed() {
    assert!(Mask::parse("  \" padded \"  ").matches(" padded "));
    assert!(!Mask::parse("  \" padded \"  ").matches("padded"));
}
