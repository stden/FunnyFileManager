//! Purpose: Pin the Command field: the catalog before `PATH`, the hints, the
//! minimum prefix for `PATH` programs, and smartcase matching.

use super::*;

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
        assert_ne!(rows.len(), 0, "{typed:?}");
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
