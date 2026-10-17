//! Purpose: unit tests for Far's `!` symbols expanded against one entry and the
//! selection.

use super::*;

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
