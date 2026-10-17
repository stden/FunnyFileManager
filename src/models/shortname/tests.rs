//! Unit tests: the 8.3 generator follows Windows's rules and never hands two
//! entries of one listing the same short name.

use super::{ShortName, ShortNames};
use crate::models::DirEntry;

fn short(names: &[&str]) -> Vec<String> {
    let table = ShortNames::for_listing(names.iter().copied());
    names
        .iter()
        .map(|name| table.get(name).expect("every listed name").to_string())
        .collect()
}

#[test]
fn a_short_name_reads_back_as_it_was_built() {
    assert_eq!(ShortName::new("LONGNA~1.EXT").as_str(), "LONGNA~1.EXT");
    assert_eq!(ShortName::new("LONGNA~1.EXT").to_string(), "LONGNA~1.EXT");
}

#[test]
fn a_name_that_already_fits_is_only_uppercased() {
    assert_eq!(
        short(&["main.rs", "Makefile", "A1_B-2.TXT", "x"]),
        ["MAIN.RS", "MAKEFILE", "A1_B-2.TXT", "X"]
    );
}

#[test]
fn a_long_stem_keeps_six_letters_and_a_tilde_number() {
    assert_eq!(short(&["longername.ext"]), ["LONGER~1.EXT"]);
    assert_eq!(short(&["README_FIRST"]), ["README~1"]);
    // Eight letters is not long: Windows gives such a name no `~N`.
    assert_eq!(short(&["longname.ext"]), ["LONGNAME.EXT"]);
}

#[test]
fn an_extension_is_cut_to_three_and_inner_dots_vanish() {
    assert_eq!(
        short(&["index.html", "archive.tar.gz", "a.b.c"]),
        ["INDEX~1.HTM", "ARCHIV~1.GZ", "AB~1.C"]
    );
}

#[test]
fn spaces_are_dropped_and_odd_characters_become_underscores() {
    assert_eq!(
        short(&["My Documents", "a+b.txt", "résumé.doc", "x y.z"]),
        ["MYDOCU~1", "A_B~1.TXT", "R_SUM_~1.DOC", "XY~1.Z"]
    );
}

#[test]
fn leading_dots_are_stripped_as_windows_does() {
    assert_eq!(
        short(&[".gitignore", ".bashrc", ".."]),
        ["GITIGN~1", "BASHRC~1", ".."]
    );
}

#[test]
fn twins_are_numbered_in_listing_order() {
    assert_eq!(
        short(&[
            "longname1.ext",
            "longname2.ext",
            "longname3.ext",
            "other.ext"
        ]),
        ["LONGNA~1.EXT", "LONGNA~2.EXT", "LONGNA~3.EXT", "OTHER.EXT"]
    );
}

#[test]
fn a_two_digit_number_costs_the_stem_a_letter() {
    let names: Vec<String> = (0..11).map(|i| format!("longname{i:02}.ext")).collect();
    let listing: Vec<&str> = names.iter().map(String::as_str).collect();
    let shorts = short(&listing);
    assert_eq!(shorts[8], "LONGNA~9.EXT");
    assert_eq!(shorts[9], "LONGN~10.EXT");
    assert_eq!(shorts[10], "LONGN~11.EXT");
}

#[test]
fn a_literal_short_name_keeps_its_slot() {
    assert_eq!(
        short(&["LONGNA~1.EXT", "longnamed.ext"]),
        ["LONGNA~1.EXT", "LONGNA~2.EXT"]
    );
}

#[test]
fn names_that_differ_only_by_case_stay_distinct() {
    assert_eq!(
        short(&["readme.md", "README.MD"]),
        ["README.MD", "README~1.MD"]
    );
}

#[test]
fn every_short_name_in_a_listing_is_unique() {
    let names: Vec<String> = (0..300)
        .map(|i| format!("a very long name number {i}.markdown"))
        .collect();
    let listing: Vec<&str> = names.iter().map(String::as_str).collect();
    let shorts = short(&listing);
    let distinct: std::collections::BTreeSet<&String> = shorts.iter().collect();
    assert_eq!(distinct.len(), shorts.len());
    assert!(shorts
        .iter()
        .all(|s| s.rsplit_once('.').map(|(_, ext)| ext) == Some("MAR")));
}

#[test]
fn an_unlisted_name_has_no_short_form() {
    let table = ShortNames::for_listing(["a.txt"]);
    assert!(table.get("b.txt").is_none());
}

#[test]
fn a_table_can_be_built_from_panel_entries() {
    let entries = vec![
        DirEntry {
            name: "..".to_owned(),
            ..DirEntry::default()
        },
        DirEntry {
            name: "Cargo.toml".to_owned(),
            ..DirEntry::default()
        },
    ];
    let table = ShortNames::for_entries(&entries);
    assert_eq!(table.get("..").map(ShortName::as_str), Some(".."));
    assert_eq!(
        table.get("Cargo.toml").map(ShortName::as_str),
        Some("CARGO~1.TOM")
    );
}
