//! Unit tests for Codex's TOML config: the same matrix as the JSON tests, plus
//! the TOML shapes a person writes by hand — comments, inline tables and
//! dotted keys.

use super::tests::{absolute, all_projects, current_folder, folder, server};
use super::*;

fn toml_state(text: &str, desired: &ServerEntry) -> ConfigState {
    state(ConfigFormat::Toml, text, Some(desired))
}

fn upserted(text: &str, desired: &ServerEntry) -> String {
    upsert(ConfigFormat::Toml, text, desired).unwrap()
}

/// `command = "<server>"` as TOML writes it.
fn command_line() -> String {
    format!("command = {:?}", server().as_str())
}

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
    assert!(rooted.contains(&format!("FFM_MCP_ROOT = {:?}", folder().as_str())));
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

#[test]
fn a_file_that_does_not_parse_is_never_rewritten() {
    assert!(matches!(
        upsert(ConfigFormat::Toml, "[mcp_servers", &all_projects()),
        Err(Blocker::Unparsable(_))
    ));
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
