//! Unit tests: Far's `!` symbols, expanded against one entry and the whole
//! selection, and the prompt's field edits.

use std::path::{Path, PathBuf};

use super::{substitute, wants_list_file, ApplyPrompt, Selection};

fn selection(names: &[&str]) -> Selection {
    Selection {
        names: names.iter().map(|name| (*name).to_owned()).collect(),
        list_file: None,
    }
}

fn entry(name: &str) -> PathBuf {
    Path::new("/work/src").join(name)
}

fn expand(template: &str) -> String {
    substitute(template, &entry("main.rs"), &selection(&["main.rs"]))
}

fn two_panels() -> super::Context {
    let mut context = super::Context::single(&entry("long name.rs"), &["long name.rs".into()]);
    context.left.entry.short = "LONGNA~1.RS".into();
    context.left.selected[0].short = "LONGNA~1.RS".into();
    context.left.description = "source code".into();
    context.right.entry = super::Names::same("other.txt");
    context.right.selected = vec![super::Names::same("other.txt")];
    context.right.paths.logical = super::Names::same("/destination");
    context
}

fn contextual(template: &str, context: &super::Context) -> String {
    super::expand(template, context, |_, _| Ok::<_, ()>("list".into())).expect("expanded")
}

#[test]
fn short_names_are_supplied_by_the_filesystem_adapter() {
    assert_eq!(
        contextual("!-! !~ !`~ !+! !&~q", &two_panels()),
        "LONGNA~1.RS LONGNA~1 RS LONGNA~1.RS LONGNA~1.RS"
    );
}

#[test]
fn panel_switches_persist_and_left_right_are_absolute() {
    let mut context = two_panels();
    assert_eq!(
        contextual("!#!.! !` !^!.! !?!", &context),
        "other.txt txt long name.rs source code"
    );
    context.active = crate::models::PanelSide::Right;
    assert_eq!(
        contextual("![!.! !]!.! !#!.! !^!.!", &context),
        "long name.rs other.txt long name.rs other.txt"
    );
}

#[test]
fn inserted_bangs_and_legacy_braces_are_literal() {
    let mut context = two_panels();
    context.left.entry.long = "hello!.txt".into();
    assert_eq!(contextual("!.! {} !!", &context), "hello!.txt {} !");
}

#[test]
fn list_modifiers_and_the_selected_panel_reach_the_writer() {
    let context = two_panels();
    let command = super::expand("!#!$QSFUW!", &context, |panel, options| {
        assert_eq!(panel.entry.long, "other.txt");
        assert_eq!(options.form, super::NameForm::Short);
        assert_eq!(options.quote, super::symbols::QuoteMode::Always);
        assert!(options.full_paths && options.forward_slashes);
        assert_eq!(options.encoding, super::ListEncoding::Utf16);
        Ok::<_, ()>("written.lst".into())
    })
    .expect("list");
    assert_eq!(command, "written.lst");
}

#[test]
fn logical_and_resolved_paths_keep_their_native_short_variants() {
    let mut context = two_panels();
    context.left.paths.logical.short = "/SHORT".into();
    context.left.paths.resolved = super::Names {
        long: "/real folder".into(),
        short: "/REALFO~1".into(),
    };
    context.left.paths.drive = "C:".into();
    let slash = std::path::MAIN_SEPARATOR;
    assert_eq!(
        contextual("!/ !=\\ !=/ !:", &context),
        format!("/SHORT{slash} /real folder{slash} /REALFO~1{slash} C:")
    );
}

#[test]
fn zz_probe_ansi_list_modifier() {
    let context = two_panels();
    let command = super::expand("!$A!", &context, |_, options| {
        assert_eq!(options.encoding, super::ListEncoding::Ansi);
        assert_eq!(options.form, super::NameForm::Short);
        Ok::<_, ()>("ansi.lst".into())
    })
    .expect("list");
    assert_eq!(command, "ansi.lst");
}

mod tests_prompt;
mod tests_symbols;
