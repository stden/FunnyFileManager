//! Purpose: TOML configs FFM cannot use or finds empty: files that do not
//! parse, keys that are not tables, and an empty servers table.

use super::*;

#[test]
fn empty_and_whitespace_only_files_are_not_configured() {
    for text in ["", "  \n\t"] {
        assert_eq!(
            toml_state(text, &all_projects()),
            ConfigState::NotConfigured
        );
    }
}

#[test]
fn a_file_that_does_not_parse_names_the_line_and_column_on_one_line() {
    let ConfigState::Blocked(Blocker::Unparsable(message)) =
        toml_state("model = \"o3\"\n[mcp_servers\n", &all_projects())
    else {
        panic!("expected a parse error");
    };
    assert!(message.contains("line 2"), "{message}");
    assert!(message.contains("column"), "{message}");
    assert!(!message.contains('\n'), "{message}");
}

#[test]
fn unexpected_shapes_name_the_key_that_is_not_a_table() {
    let cases = [
        ("mcp_servers = 3", "mcp_servers"),
        ("[mcp_servers]\nffm-mcp = \"x\"", "mcp_servers.ffm-mcp"),
        (
            "[mcp_servers.ffm-mcp]\ncommand = \"x\"\nenv = 3",
            "mcp_servers.ffm-mcp.env",
        ),
    ];
    for (text, key) in cases {
        let blocked = ConfigState::Blocked(Blocker::UnexpectedLayout(key));
        assert_eq!(toml_state(text, &all_projects()), blocked, "{text}");
        assert_eq!(
            upsert(ConfigFormat::Toml, text, &current_folder()),
            Err(Blocker::UnexpectedLayout(key)),
            "{text}"
        );
    }
}

#[test]
fn a_root_removal_from_an_env_that_is_not_a_table_is_refused() {
    let text = "[mcp_servers.ffm-mcp]\ncommand = \"x\"\nenv = 3";
    assert_eq!(
        upsert(ConfigFormat::Toml, text, &all_projects()),
        Err(Blocker::UnexpectedLayout("mcp_servers.ffm-mcp.env"))
    );
}

#[test]
fn an_empty_servers_table_is_not_configured() {
    assert_eq!(
        toml_state("[mcp_servers]\n", &all_projects()),
        ConfigState::NotConfigured
    );
}

#[test]
fn a_file_that_does_not_parse_is_never_rewritten() {
    assert!(matches!(
        upsert(ConfigFormat::Toml, "[mcp_servers", &all_projects()),
        Err(Blocker::Unparsable(_))
    ));
}
