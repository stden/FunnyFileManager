//! Purpose: Setting the TOML entry: a new file's section, and the comments,
//! inline tables, dotted keys, extra keys and Windows paths that survive it.

use super::*;

#[test]
fn a_new_file_gains_only_the_entry_section() {
    let written = upserted("", &all_projects());
    assert_eq!(
        written,
        format!("[mcp_servers.ffm-mcp]\n{}\nargs = []\n", command_line())
    );
    let rooted = upserted("", &current_folder());
    assert!(rooted.contains("[mcp_servers.ffm-mcp.env]"), "{rooted}");
    assert_eq!(
        toml_state(&rooted, &current_folder()),
        ConfigState::Registered
    );
    assert!(
        rooted.contains(&format!(
            "FFM_MCP_ROOT = {}",
            toml_string(folder().as_str())
        )),
        "{rooted}"
    );
}

#[test]
fn comments_blank_lines_and_other_servers_survive() {
    let text = "# my Codex setup\nmodel = \"o3\"  # the fast one\n\n\
                [mcp_servers.other]\ncommand = \"other\"\n\n# trailing note\n";
    let written = upserted(text, &all_projects());
    let (head, _) = text.split_once("\n\n# trailing").unwrap();
    assert!(written.starts_with(head), "{written}");
    assert!(written.ends_with("\n# trailing note\n"), "{written}");
    assert!(written.contains("[mcp_servers.ffm-mcp]"), "{written}");
    assert_eq!(
        toml_state(&written, &all_projects()),
        ConfigState::Registered
    );
}

#[test]
fn an_inline_servers_table_stays_inline() {
    let text = "mcp_servers = { other = { command = \"o\" } }\n";
    let written = upserted(text, &current_folder());
    assert!(!written.contains("[mcp_servers"), "{written}");
    assert!(written.contains("other = { command = \"o\" }"), "{written}");
    assert_eq!(
        toml_state(&written, &current_folder()),
        ConfigState::Registered
    );
}

#[test]
fn dotted_keys_are_updated_in_place() {
    let text =
        "mcp_servers.ffm-mcp.command = \"/old\"\nmcp_servers.ffm-mcp.startup_timeout_sec = 20\n";
    let written = upserted(text, &all_projects());
    assert!(written.contains("startup_timeout_sec = 20"), "{written}");
    assert!(!written.contains("/old"), "{written}");
    assert_eq!(
        toml_state(&written, &all_projects()),
        ConfigState::Registered
    );
}

#[test]
fn extra_keys_survive_and_the_root_goes_at_all_projects() {
    let text =
        "[mcp_servers.ffm-mcp]\ncommand = \"/old\"\nargs = [\"x\"]\nstartup_timeout_sec = 20\n\
                [mcp_servers.ffm-mcp.env]\nRUST_LOG = \"debug\"\nFFM_MCP_ROOT = \"/srv/x/other\"\n";
    let written = upserted(text, &all_projects());
    assert!(written.contains("startup_timeout_sec = 20"), "{written}");
    assert!(written.contains("RUST_LOG = \"debug\""), "{written}");
    assert!(!written.contains("FFM_MCP_ROOT"), "{written}");
    assert!(written.contains("args = []"), "{written}");
}

#[test]
fn windows_paths_with_backslashes_round_trip() {
    let command = ServerBinary::new(PathBuf::from(r"C:\Tools\bin\ffm-mcp.exe")).unwrap();
    let desired = ServerEntry {
        command,
        root: None,
    };
    let written = upserted("", &desired);
    assert_eq!(
        toml_state(&written, &desired),
        ConfigState::Registered,
        "{written}"
    );
    let literal = "[mcp_servers.ffm-mcp]\ncommand = 'C:\\Tools\\bin\\ffm-mcp.exe'\n";
    assert_eq!(toml_state(literal, &desired), ConfigState::Registered);
}

#[test]
fn upsert_is_idempotent() {
    let inputs = [
        String::new(),
        "model = \"o3\"\n".to_owned(),
        "[mcp_servers.ffm-mcp]\ncommand = \"/old\"\nargs = [\"x\"]\n".to_owned(),
        format!(
            "mcp_servers.ffm-mcp.command = {:?}",
            absolute("/srv/x/other").display().to_string()
        ),
    ];
    for text in &inputs {
        for desired in [all_projects(), current_folder()] {
            let once = upserted(text, &desired);
            assert_eq!(upserted(&once, &desired), once, "{text}");
            assert_eq!(
                toml_state(&once, &desired),
                ConfigState::Registered,
                "{once}"
            );
        }
    }
}
