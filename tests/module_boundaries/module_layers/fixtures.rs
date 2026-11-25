//! Purpose: Planted layer maps and sources, one per message the layer rule makes.

use super::super::architecture::SourceFile;
use super::{cyclic_modules, layer_violations, LayerMap};
use std::collections::BTreeSet;

const LAYERS: [(&str, &[&str]); 3] = [("low", &["a", "b"]), ("mid", &["c"]), ("high", &["d"])];

fn map(debt: &'static [(&'static str, &'static str)]) -> LayerMap<'static> {
    LayerMap {
        layers: &LAYERS,
        ignored: &["ignored"],
        debt,
    }
}

/// Every layered module exists unless a case plants it: a, b, c and d are empty by default.
fn tree(files: &[(&str, &str)]) -> Vec<SourceFile> {
    let mut all: Vec<(&str, &str)> = [
        ("src/a.rs", ""),
        ("src/b.rs", ""),
        ("src/c.rs", ""),
        ("src/d.rs", ""),
    ]
    .into_iter()
    .filter(|(path, _)| !files.iter().any(|(given, _)| given == path))
    .collect();
    all.extend_from_slice(files);
    all.iter()
        .map(|(relative, text)| SourceFile::new(relative, text))
        .collect()
}

#[test]
fn an_upward_use_is_reported_with_its_place() {
    let sources = tree(&[("src/a.rs", "fn f() {\n    crate::d::run();\n}\n")]);
    assert_eq!(
        layer_violations(&sources, &map(&[])),
        ["src/a.rs:2: `crate::d::run` (a in low uses d in high, a higher layer; named 1 time(s))"]
    );
}

#[test]
fn a_listed_use_is_tolerated_and_a_gone_one_is_reported() {
    const DEBT: &[(&str, &str)] = &[("a", "d"), ("c", "d")];
    let sources = tree(&[("src/a.rs", "fn f() { crate::d::run(); }\n")]);
    assert_eq!(
        layer_violations(&sources, &map(DEBT)),
        ["debt `c` -> `d` is gone: remove it"]
    );
}

#[test]
fn a_module_in_no_layer_is_reported() {
    let sources = tree(&[("src/mystery.rs", "")]);
    assert_eq!(
        layer_violations(&sources, &map(&[])),
        ["module `mystery` is in no layer"]
    );
}

#[test]
fn a_crate_root_alias_is_followed_to_the_module_that_defines_it() {
    let sources = tree(&[
        ("src/lib.rs", "mod d;\npub use d::run as go;\n"),
        ("src/a.rs", "fn f() {\n    crate::go();\n}\n"),
    ]);
    assert_eq!(
        layer_violations(&sources, &map(&[])),
        ["src/a.rs:2: `crate::d::run` (a in low uses d in high, a higher layer; named 1 time(s))"]
    );
}

#[test]
fn a_use_group_with_a_nested_group_still_names_the_modules_after_it() {
    let sources = tree(&[("src/a.rs", "use crate::{\n    b::{X, Y},\n    d::Z,\n};\n")]);
    let messages = layer_violations(&sources, &map(&[]));
    assert!(
        messages
            .iter()
            .any(|message| message.contains("`crate::d::Z`")),
        "{messages:?}"
    );
}

#[test]
fn comments_strings_and_test_code_are_not_references() {
    let sources = tree(&[(
        "src/a.rs",
        "// crate::d::run is only named in a comment\nconst TEXT: &str = \"crate::d::run\";\n#[cfg(test)]\nmod tests {\n    use crate::d::run;\n}\n",
    )]);
    assert_eq!(layer_violations(&sources, &map(&[])), Vec::<String>::new());
}

#[test]
fn ignored_modules_are_not_checked() {
    let sources = tree(&[("src/ignored.rs", "fn f() { crate::d::run(); }\n")]);
    assert_eq!(layer_violations(&sources, &map(&[])), Vec::<String>::new());
}

#[test]
fn a_module_listed_in_two_layers_is_reported() {
    const TWICE: [(&str, &[&str]); 2] = [("low", &["a"]), ("high", &["a"])];
    let map = LayerMap {
        layers: &TWICE,
        ignored: &[],
        debt: &[],
    };
    let sources = vec![SourceFile::new("src/a.rs", "")];
    assert_eq!(
        layer_violations(&sources, &map),
        ["module `a` is in two layers"]
    );
}

#[test]
fn a_layer_entry_with_no_module_is_reported() {
    const GHOST: [(&str, &[&str]); 1] = [("low", &["a", "ghost"])];
    let map = LayerMap {
        layers: &GHOST,
        ignored: &[],
        debt: &[],
    };
    let sources = vec![SourceFile::new("src/a.rs", "")];
    assert_eq!(
        layer_violations(&sources, &map),
        ["layer entry `ghost` names no module in src/"]
    );
}

#[test]
fn a_loop_inside_one_layer_is_found_and_a_chain_is_not() {
    let sources = tree(&[
        ("src/a.rs", "fn f() { crate::b::g(); }\n"),
        ("src/b.rs", "fn g() { crate::a::f(); }\n"),
        ("src/c.rs", "fn h() { crate::d::run(); }\n"),
    ]);
    let expected: BTreeSet<String> = ["a", "b"]
        .iter()
        .map(|module| (*module).to_owned())
        .collect();
    assert_eq!(cyclic_modules(&sources, &map(&[])), expected);
}
