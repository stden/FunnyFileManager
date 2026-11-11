//! Purpose: The state a TOML config reads as: inline tables and dotted keys
//! as tables, a different entry, disabled, or started some other way.

use super::*;

#[test]
fn inline_tables_and_dotted_keys_are_read_as_tables() {
    let inline = format!("mcp_servers = {{ ffm-mcp = {{ {} }} }}", command_line());
    let dotted = format!("mcp_servers.ffm-mcp.{}", command_line());
    let section = format!(
        "[mcp_servers]\nffm-mcp = {{ {}, args = [] }}",
        command_line()
    );
    for text in [inline, dotted, section] {
        assert_eq!(
            toml_state(&text, &all_projects()),
            ConfigState::Registered,
            "{text}"
        );
    }
}

#[test]
fn another_command_arguments_or_root_is_a_different_entry() {
    let root = format!(
        "{}\n[mcp_servers.ffm-mcp.env]\nFFM_MCP_ROOT = \"/srv/x/other\"",
        command_line()
    );
    let texts = [
        "[mcp_servers.ffm-mcp]\ncommand = \"/old/ffm-mcp\"".to_owned(),
        format!("[mcp_servers.ffm-mcp]\n{}\nargs = [\"-v\"]", command_line()),
        format!("[mcp_servers.ffm-mcp]\n{root}"),
    ];
    for text in texts {
        assert_eq!(
            toml_state(&text, &all_projects()),
            ConfigState::DifferentEntry,
            "{text}"
        );
    }
}

/// Review finding: Codex's `enabled = false` read as plain "registered".
#[test]
fn a_switched_off_entry_is_registered_but_disabled() {
    for switch in ["enabled = false", "disabled = true"] {
        let text = format!("[mcp_servers.ffm-mcp]\n{}\n{switch}\n", command_line());
        assert_eq!(
            toml_state(&text, &all_projects()),
            ConfigState::Disabled,
            "{text}"
        );
    }
    let on = format!(
        "[mcp_servers.ffm-mcp]\n{}\nenabled = true\n",
        command_line()
    );
    assert_eq!(toml_state(&on, &all_projects()), ConfigState::Registered);
}

/// Review finding: a `cwd` pins the server to one folder and a `url` is not
/// this server at all, yet both read as registered.
#[test]
fn an_entry_started_some_other_way_is_a_different_entry_until_set() {
    for extra in ["cwd = \"/srv/x/one\"", "url = \"http://localhost:1\""] {
        let text = format!(
            "# mine\n[mcp_servers.ffm-mcp]\n{}\n{extra}\nstartup_timeout_sec = 20\n",
            command_line()
        );
        assert_eq!(
            toml_state(&text, &all_projects()),
            ConfigState::DifferentEntry,
            "{text}"
        );
        let written = upserted(&text, &all_projects());
        assert_eq!(
            toml_state(&written, &all_projects()),
            ConfigState::Registered,
            "{written}"
        );
        assert!(!written.contains(extra), "{written}");
        assert!(written.starts_with("# mine\n"), "{written}");
        assert!(written.contains("startup_timeout_sec = 20"), "{written}");
    }
}
