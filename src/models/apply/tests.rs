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

/// `!.!` is the name; a bare `!` is the name with its extension cut off, so
/// `cp !.! !.bak` is Far's own idiom for a backup copy.
#[test]
fn the_name_with_and_without_its_extension() {
    assert_eq!(expand("cp !.! !.bak"), "cp main.rs main.bak");
}

/// Short-name substitutions fall back to the real name when no alias exists.
#[test]
fn far_distinguishes_short_names_from_extensions() {
    assert_eq!(expand("!~"), "main");
    assert_eq!(expand("!`"), "rs");
    assert_eq!(expand("!`~"), "rs");
    assert_eq!(expand("!-!"), "main.rs");
    assert_eq!(expand("!+!"), "main.rs");
}

/// A bare `!` is greedy in Far too: `!x` is the stem followed by an `x`.
#[test]
fn a_bare_bang_binds_to_the_stem() {
    assert_eq!(expand("!x"), "mainx");
}

/// `!!` is the only way to get a literal `!` into the command — and it is
/// consumed as a pair, so the `!` after `!!.` is once more the bare stem.
#[test]
fn a_double_bang_is_a_literal_bang() {
    assert_eq!(expand("echo !!"), "echo !");
    assert_eq!(expand("!!.!"), "!.main");
}

/// Far splits at the last dot even when it is the first character.
#[test]
fn a_dotfile_uses_fars_last_dot_rule() {
    let dotfile = entry(".gitignore");
    let selected = selection(&[".gitignore"]);
    assert_eq!(substitute("!", &dotfile, &selected), "");
    assert_eq!(substitute("!`", &dotfile, &selected), "gitignore");
    assert_eq!(substitute("!.!", &dotfile, &selected), ".gitignore");
}

/// Both directory substitutions include the separator needed before `!.!`.
#[test]
fn far_paths_can_be_concatenated_with_a_filename() {
    let expected = entry("main.rs").display().to_string();
    assert_eq!(expand("!\\!.!"), expected);
    assert_eq!(expand("!/!.!"), expected);
}

/// `!&` lists every selected name, each quoted (Far's default since build
/// 5466); `!&q` keeps only the quotes required around whitespace.
#[test]
fn the_selection_quoted_unless_asked_bare() {
    let selected = selection(&["a b.txt", "c.txt"]);
    let main = entry("a b.txt");
    assert_eq!(
        substitute("tar cf x.tar !&", &main, &selected),
        "tar cf x.tar \"a b.txt\" \"c.txt\""
    );
    assert_eq!(substitute("!&Q", &main, &selected), "\"a b.txt\" \"c.txt\"");
    assert_eq!(substitute("!&q", &main, &selected), "\"a b.txt\" c.txt");
}

/// Far requires the closing exclamation mark for a list-file substitution.
#[test]
fn list_files_require_the_far_terminator() {
    let mut selected = selection(&["a.txt"]);
    selected.list_file = Some(PathBuf::from("/tmp/ffm-apply.lst"));
    let main = entry("a.txt");
    assert_eq!(substitute("cat !@", &main, &selected), "cat a@");
    assert_eq!(
        substitute("cat !@!", &main, &selected),
        "cat /tmp/ffm-apply.lst"
    );
    assert!(!wants_list_file("xargs < !@"));
    assert!(wants_list_file("cat !@! | sort"));
    assert!(wants_list_file("cat !@UFQ!"));
    assert!(wants_list_file("cat !$!"));
    assert!(!wants_list_file("echo !!@!"));
    assert!(!wants_list_file("echo !.!"));
}

/// A template that names the list file but was given none expands it to
/// nothing rather than to a path that does not exist.
#[test]
fn a_missing_list_file_expands_to_nothing() {
    assert_eq!(expand("cat !@!"), "cat ");
}

/// The prompt edits like every other single-field dialog.
#[test]
fn the_prompt_edits_its_field_and_moves_between_buttons() {
    let mut prompt = ApplyPrompt::default();
    assert!(!prompt.on_cancel());
    prompt.input_mut().insert_str("lsx");
    prompt.input_mut().backspace();
    assert_eq!(prompt.command(), "ls");
    assert_eq!(prompt.input().after_cursor(), "");
    prompt.toggle_button();
    assert!(prompt.on_cancel());
    prompt.toggle_button();
    assert!(!prompt.on_cancel());
    prompt.focus_cancel(true);
    assert!(prompt.on_cancel());
    prompt.focus_cancel(false);
    assert!(!prompt.on_cancel());
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
