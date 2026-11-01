//! Purpose: `sessions` and `sessions import netbox [path]`, and the usage for
//! anything else.
use super::*;

/// `:sessions` opens the panel; `:sessions import netbox [path]` imports,
/// the path being the rest of the line.
#[test]
fn sessions_parses_its_verb_the_import_and_a_path_with_spaces() {
    use super::SessionsCommand;
    assert_eq!(
        Command::parse("sessions"),
        Command::Sessions(SessionsCommand::Open)
    );
    assert_eq!(
        Command::parse("sessions import netbox"),
        Command::Sessions(SessionsCommand::ImportNetBox(None))
    );
    assert_eq!(
        Command::parse("  sessions   import  netbox  \"/srv/x/my host.netbox\"  "),
        Command::Sessions(SessionsCommand::ImportNetBox(Some(PathBuf::from(
            "/srv/x/my host.netbox"
        ))))
    );
    assert!(VERBS.contains(&"sessions"));
}

/// Anything else after `sessions` is the usage.
#[test]
fn sessions_with_other_words_is_its_usage() {
    for bad in [
        "sessions import",
        "sessions import far",
        "sessions import NETBOX x",
        "sessions open",
    ] {
        assert!(
            matches!(
                Command::parse(bad),
                Command::Invalid { usage, .. }
                    if usage == "sessions expects nothing, or import netbox [path]"
            ),
            "{bad}"
        );
    }
}
