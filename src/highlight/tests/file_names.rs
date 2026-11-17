//! Purpose: language detection from file names: extensions, extensionless names, dotenv variants and config files.

use super::*;

#[test]
fn an_unknown_extension_is_left_plain() {
    let mut highlighter = Highlighter::new(&PathBuf::from("notes.xyz"));
    assert!(!highlighter.is_active());
    assert_eq!(
        highlighter.line("let x = 1;"),
        vec![("let x = 1;", Token::Normal)]
    );
}

/// Files whose language is in the name, not the extension.
#[test]
fn extensionless_names_still_resolve_to_a_language() {
    assert!(Highlighter::new(&PathBuf::from("/x/Dockerfile")).is_active());
    assert!(Highlighter::new(&PathBuf::from("/x/Makefile")).is_active());
    assert!(!Highlighter::new(&PathBuf::from("/x/LICENSE")).is_active());
}

#[test]
fn dotenv_names_and_variants_use_the_builtin_scanner() {
    for name in [
        ".env",
        ".env.local",
        ".env.production.local",
        ".env.example",
        "service.env",
    ] {
        let path = PathBuf::from("/project").join(name);
        let lines = vec!["export API_URL=${HOST}/api # note".to_owned()];
        assert!(syntax_for(&path).is_some(), "{name}");
        let mut highlighter = Highlighter::for_file(&path, &lines);
        assert!(highlighter.is_active(), "{name}");
        let result = highlighter.line(&lines[0]);
        assert!(
            result.contains(&("API_URL", Token::Type)),
            "{name}: {result:?}"
        );
        assert!(
            result.contains(&("${HOST}", Token::Type)),
            "{name}: {result:?}"
        );
        assert!(
            result.contains(&("# note", Token::Comment)),
            "{name}: {result:?}"
        );
    }
    assert!(!Highlighter::new(Path::new(".environment")).is_active());
}

#[test]
fn json_and_config_files_are_recognised() {
    let json = runs("a.json", r#"  "key": true"#);
    assert_eq!(token_of(&json, r#""key""#), Token::Str);
    assert_eq!(token_of(&json, "true"), Token::Keyword);
    // Leading indentation is one run, not one per blank.
    assert_eq!(json[0].0, "  ");
    assert_eq!(runs("Cargo.toml", "# a comment")[0].1, Token::Comment);
    assert_eq!(runs("app.yaml", "# a comment")[0].1, Token::Comment);
}

/// `.git/config` has no extension, so it fell through to plain text — the
/// commonest file in the repo that a user opens and expects to be coloured.
#[test]
fn extensionless_config_files_are_recognised_by_name() {
    // Far paints a section header white, like a keyword.
    assert_eq!(runs("/x/.git/config", "[core]")[0].1, Token::Keyword);
    assert_eq!(
        runs("/x/.git/config", "\trepositoryformatversion = 0")[0].1,
        Token::Normal
    );
    assert_eq!(
        token_of(&runs("/x/.git/config", "\tbare = false"), "false"),
        Token::Keyword
    );
    assert!(Highlighter::new(&PathBuf::from("/x/.gitconfig")).is_active());
    assert!(Highlighter::new(&PathBuf::from("/etc/hosts")).is_active());
}
