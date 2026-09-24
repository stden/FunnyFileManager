//! Purpose: Pin what each Language servers field offers, in which order and
//! with which hint, and how an accepted suggestion lands in the field.

use tempfile::TempDir;

use super::*;
use crate::config::ServerSettings;

/// The names a fake `PATH` directory lists, as the probe scans them.
fn scanned(names: &[&str]) -> (TempDir, Vec<String>) {
    let bin = TempDir::new().expect("bin");
    for name in names {
        let file = bin.path().join(crate::lsp::program_file(name));
        crate::test_support::write_executable(&file, b"program").expect("program");
    }
    let path = execs::join_paths([bin.path()]);
    let programs = execs::scan(path.as_deref());
    (bin, programs)
}

/// The names as the probe scans them, each from a `PATH` directory of its
/// own: names differing only in case, which one directory holds as one file
/// on Windows and macOS.
fn scanned_apart(names: &[&str]) -> (Vec<TempDir>, Vec<String>) {
    let bins: Vec<TempDir> = names.iter().map(|name| scanned(&[name]).0).collect();
    let path = execs::join_paths(bins.iter().map(TempDir::path));
    let programs = execs::scan(path.as_deref());
    (bins, programs)
}

fn texts(candidates: &[Candidate]) -> Vec<&str> {
    candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect()
}

fn hint<'a>(candidates: &'a [Candidate], text: &str) -> Option<&'a str> {
    candidates
        .iter()
        .find(|candidate| candidate.text == text)
        .and_then(|candidate| candidate.hint.as_deref())
}

fn offered(
    field: CompletedField,
    typed: &str,
    draft: &LspSettings,
    programs: &[String],
) -> Vec<Candidate> {
    let vocabulary = Vocabulary {
        catalog: LspSettings::catalog(),
        draft,
        programs,
    };
    candidates(field, &LineEditor::new(typed), &vocabulary)
}

fn file(name: &str) -> String {
    crate::lsp::program_file(name)
}

#[test]
fn a_command_offers_the_catalog_before_path_with_a_merged_hint() {
    let (_bin, programs) = scanned(&["pylsp", "pydoc3", "python3", "pyright-langserver"]);
    let rows = offered(
        CompletedField::Command,
        "py",
        LspSettings::catalog(),
        &programs,
    );
    assert_eq!(
        texts(&rows),
        [
            "pylsp".to_owned(),
            file("pydoc3"),
            file("python3"),
            file("pyright-langserver")
        ]
    );
    assert_eq!(hint(&rows, "pylsp"), Some("catalog · PATH"));
    assert_eq!(hint(&rows, &file("pydoc3")), Some("PATH"));
}

/// One letter matches hundreds of programs: an empty or one-character token
/// offers the known commands alone.
#[test]
fn a_short_token_offers_only_the_known_commands() {
    let (_bin, programs) = scanned(&["pydoc3", "perl"]);
    for typed in ["", "p"] {
        let rows = offered(
            CompletedField::Command,
            typed,
            LspSettings::catalog(),
            &programs,
        );
        assert!(!rows.is_empty(), "{typed:?}");
        assert!(
            rows.iter()
                .all(|row| row.hint.as_deref() == Some("catalog")),
            "{typed:?}: {:?}",
            texts(&rows)
        );
    }
    assert_eq!(
        offered(
            CompletedField::Command,
            "",
            LspSettings::catalog(),
            &programs
        )
        .len(),
        10,
        "every distinct catalog command and npm binary"
    );
}

#[test]
fn a_fuzzy_token_finds_the_command_and_a_draft_command_is_in_use() {
    let rows = offered(CompletedField::Command, "ra", LspSettings::catalog(), &[]);
    assert_eq!(texts(&rows).first(), Some(&"rust-analyzer"));

    let mut draft = LspSettings::catalog().clone();
    draft.0.insert(
        "fixture".to_owned(),
        ServerSettings {
            command: "/srv/x/fixture-server".to_owned(),
            ..ServerSettings::default()
        },
    );
    let rows = offered(CompletedField::Command, "fixture", &draft, &[]);
    assert_eq!(texts(&rows), ["/srv/x/fixture-server"]);
    assert_eq!(hint(&rows, "/srv/x/fixture-server"), Some("in use"));
}

/// An npm recipe installs its own binary, which the catalog knows as a
/// command even where the server's command is named otherwise.
#[test]
fn a_catalog_npm_binary_is_offered_beside_the_command() {
    let catalog = LspSettings(
        [(
            "fixture".to_owned(),
            ServerSettings {
                command: "fixture-launcher".to_owned(),
                install: Some(InstallRecipe::Npm {
                    packages: vec!["fixture-ls".to_owned()],
                    package: "fixture-ls".to_owned(),
                    binary: "fixture-ls".to_owned(),
                }),
                ..ServerSettings::default()
            },
        )]
        .into(),
    );
    let draft = LspSettings(BTreeMap::new());
    let vocabulary = Vocabulary {
        catalog: &catalog,
        draft: &draft,
        programs: &[],
    };
    let rows = candidates(
        CompletedField::Command,
        &LineEditor::new("fixture"),
        &vocabulary,
    );
    let mut offered = texts(&rows);
    offered.sort_unstable();
    assert_eq!(offered, ["fixture-launcher", "fixture-ls"]);
    assert_eq!(hint(&rows, "fixture-ls"), Some("catalog"));
}

/// An uppercase letter makes the match case-sensitive, as on the command line.
///
/// The two spellings sit in two `PATH` directories: Windows and macOS keep
/// file names case-insensitively, so in one directory the second write
/// would only replace the first file's contents.
#[test]
fn matching_is_smartcase() {
    let (_bins, programs) = scanned_apart(&["PyTool", "pytool"]);
    let upper = offered(
        CompletedField::Command,
        "Py",
        LspSettings::catalog(),
        &programs,
    );
    assert_eq!(texts(&upper), [file("PyTool")]);
    let lower = offered(
        CompletedField::Command,
        "py",
        LspSettings::catalog(),
        &programs,
    );
    assert!(texts(&lower).contains(&"pylsp"));
    assert!(texts(&lower).contains(&file("pytool").as_str()));
}

/// Windows lookup appends `.exe`: a `.dll` or an extensionless shim on `PATH`
/// would be reported missing, so it is not offered.
#[cfg(windows)]
#[test]
fn only_a_launchable_program_is_offered_on_windows() {
    let bin = TempDir::new().expect("bin");
    for name in ["pytool.exe", "pytool.dll", "pyshim"] {
        std::fs::write(bin.path().join(name), "x").expect("program");
    }
    let programs = execs::scan(execs::join_paths([bin.path()]).as_deref());
    let rows = offered(
        CompletedField::Command,
        "py",
        LspSettings::catalog(),
        &programs,
    );
    assert!(texts(&rows).contains(&"pytool.exe"));
    assert!(!texts(&rows).contains(&"pytool.dll"));
    assert!(!texts(&rows).contains(&"pyshim"));
}

#[test]
fn arguments_come_from_the_catalog_and_the_draft_only() {
    let mut draft = LspSettings::catalog().clone();
    draft.0.insert(
        "fixture".to_owned(),
        ServerSettings {
            args: vec![
                "--log=verbose".to_owned(),
                String::new(),
                "bad\u{7}".to_owned(),
            ],
            ..ServerSettings::default()
        },
    );
    let rows = offered(CompletedField::Arguments, "", &draft, &[]);
    assert_eq!(texts(&rows), ["start", "--stdio", "--log=verbose"]);
    assert_eq!(hint(&rows, "--stdio"), Some("catalog"));
    assert_eq!(hint(&rows, "--log=verbose"), Some("in use"));
    for transport in ["--socket=", "--node-ipc", "--pipe="] {
        assert!(!texts(&rows).contains(&transport), "{transport}");
    }
    let rows = offered(CompletedField::Arguments, r#"--stdio "--l"#, &draft, &[]);
    assert_eq!(texts(&rows), ["--log=verbose"]);
}

#[test]
fn extensions_name_their_owners_skip_the_field_and_carry_a_dot() {
    let mut draft = LspSettings::catalog().clone();
    draft.0.insert(
        "deno".to_owned(),
        ServerSettings {
            extensions: vec!["ts".to_owned()],
            ..ServerSettings::default()
        },
    );
    let rows = offered(CompletedField::Extensions, ".t", &draft, &[]);
    assert_eq!(hint(&rows, ".ts"), Some("deno, typescript"));
    assert_eq!(hint(&rows, ".tsx"), Some("typescriptreact"));
    assert!(texts(&rows).iter().all(|text| text.starts_with('.')));

    let rows = offered(CompletedField::Extensions, "TS, .t", &draft, &[]);
    assert!(!texts(&rows).contains(&".ts"), "already in the field");
    assert!(texts(&rows).contains(&".tsx"));
}

/// The word the caret is in is being retyped, so the part of it after the
/// caret is still offered; the words after it are in the field already.
#[test]
fn the_rest_of_the_word_under_the_caret_is_still_offered() {
    let vocabulary = Vocabulary {
        catalog: LspSettings::catalog(),
        draft: LspSettings::catalog(),
        programs: &[],
    };
    let rows = candidates(
        CompletedField::Extensions,
        &line_at("", "ts rs"),
        &vocabulary,
    );
    assert!(texts(&rows).contains(&".ts"), "{:?}", texts(&rows));
    assert!(!texts(&rows).contains(&".rs"), "{:?}", texts(&rows));
}

#[test]
fn the_typed_token_is_spelled_like_a_candidate() {
    let line = |text: &str| LineEditor::new(text);
    assert_eq!(typed(CompletedField::Extensions, &line("rs, py")), ".py");
    assert_eq!(typed(CompletedField::Extensions, &line("rs ")), "");
    assert_eq!(typed(CompletedField::Arguments, &line(r#"a "b c"#)), "b c");
    assert_eq!(typed(CompletedField::Command, &line("  pylsp")), "pylsp");
}

/// A line with the caret after `before` and `after` following it.
fn line_at(before: &str, after: &str) -> LineEditor {
    let mut line = LineEditor::new("");
    line.set_split(before, after);
    line
}

#[test]
fn an_accepted_command_replaces_the_token_without_a_space() {
    let mut line = line_at("py", "");
    accept(CompletedField::Command, &mut line, "pylsp");
    assert_eq!(line.text(), "pylsp");
    assert_eq!(line.before_cursor(), "pylsp");
}

#[test]
fn an_accepted_argument_is_quoted_when_it_must_be_and_spaced() {
    let mut line = line_at("--stdio --lo", "");
    accept(CompletedField::Arguments, &mut line, "--log=a b");
    assert_eq!(line.text(), r#"--stdio "--log=a b" "#);

    let mut line = line_at("st", " --verbose");
    accept(CompletedField::Arguments, &mut line, "start");
    assert_eq!(line.text(), "start --verbose");
    assert_eq!(line.after_cursor(), " --verbose");
}

#[test]
fn an_accepted_extension_replaces_the_word_with_its_dotted_form() {
    let mut line = line_at("rs, p", "");
    accept(CompletedField::Extensions, &mut line, ".pyi");
    assert_eq!(line.text(), "rs, .pyi ");

    let mut line = line_at(".p", " .rs");
    accept(CompletedField::Extensions, &mut line, ".py");
    assert_eq!(line.text(), ".py .rs");
}

#[test]
fn the_path_pool_needs_the_minimum_prefix() {
    assert_eq!(MIN_PATH_PREFIX, 2);
    let (_bin, programs) = scanned(&["zz-tool"]);
    let short = offered(
        CompletedField::Command,
        "z",
        LspSettings::catalog(),
        &programs,
    );
    assert_eq!(texts(&short), ["rust-analyzer"], "the catalog only");
    let long = offered(
        CompletedField::Command,
        "zz",
        LspSettings::catalog(),
        &programs,
    );
    assert_eq!(
        texts(&long),
        [file("zz-tool")],
        "no catalog command holds zz"
    );
}

/// A no-break space (Option+Space on macOS) or an ideographic space (CJK input
/// methods) separates extensions: the word starts after its last byte, never
/// inside it, whether the list is offered, the token spelled or one accepted.
#[test]
fn a_multibyte_space_ends_the_extension_word_after_its_last_byte() {
    for space in ['\u{a0}', '\u{3000}'] {
        let before = format!(".rs{space}p");
        assert_eq!(
            typed(CompletedField::Extensions, &line_at(&before, "")),
            ".p"
        );
        let rows = offered(
            CompletedField::Extensions,
            &before,
            LspSettings::catalog(),
            &[],
        );
        assert!(texts(&rows).contains(&".py"), "{space:?}");
        assert!(!texts(&rows).contains(&".rs"), "already in the field");

        let mut line = line_at(&before, "");
        accept(CompletedField::Extensions, &mut line, ".py");
        assert_eq!(line.text(), format!(".rs{space}.py "));

        let just_after = format!(".rs{space}");
        assert_eq!(
            typed(CompletedField::Extensions, &line_at(&just_after, "")),
            ""
        );
    }
}
