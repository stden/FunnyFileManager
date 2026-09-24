//! Unit tests for [`super`]: the exact `claude` calls, the check after them,
//! and every way the CLI can fail — against the emulator, never a real CLI.

use std::fs;

use crate::mcp_clients::{Captured, ProjectDir, Scope, ServerBinary};
use crate::test_support::{ClaudeBehaviour, FakeMachine};

use super::*;

struct Fixture {
    machine: FakeMachine,
    host: Host,
    desired: ServerEntry,
}

fn fixture() -> Fixture {
    let machine = FakeMachine::new().expect("fake machine");
    machine.install("claude").unwrap();
    let server = ServerBinary::new(machine.install_server().unwrap()).unwrap();
    let host = machine.host();
    Fixture {
        machine,
        host,
        desired: ServerEntry {
            command: server,
            root: None,
        },
    }
}

impl Fixture {
    fn register(&self) -> Registration {
        register(&self.host, &self.desired)
    }

    fn call(&self, args: &[&str]) -> CliCall {
        CliCall {
            program: self.machine.bin().join("claude"),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            cwd: self.machine.home(),
            timeout: Duration::from_secs(30),
        }
    }

    fn add(&self) -> CliCall {
        let server = self.desired.command.as_str();
        self.call(&[
            "mcp",
            "add",
            "--scope",
            "user",
            "--transport",
            "stdio",
            "ffm-mcp",
            "--",
            server,
        ])
    }

    fn remove(&self) -> CliCall {
        self.call(&["mcp", "remove", "--scope", "user", "ffm-mcp"])
    }

    /// `claude mcp add-json` with the entry the user config should end with.
    fn add_json(&self, entry: &serde_json::Value) -> CliCall {
        let entry = entry.to_string();
        self.call(&["mcp", "add-json", "--scope", "user", "ffm-mcp", &entry])
    }

    /// The entry `write_entry(command)` becomes once this server is set.
    fn replaced(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "stdio", "command": self.desired.command.as_str(), "args": [], "env": {}
        })
    }

    fn write_entry(&self, command: &str) {
        let entry = serde_json::json!({"oauthAccount": {"id": "x"}, "mcpServers": {"ffm-mcp": {
            "type": "stdio", "command": command, "args": [], "env": {}
        }}});
        fs::write(self.machine.home().join(".claude.json"), entry.to_string()).unwrap();
    }
}

#[test]
fn an_absent_entry_is_added_with_one_call_in_the_home_folder() {
    let fixture = fixture();
    let registration = fixture.register();
    assert_eq!(registration.outcome, Outcome::Added);
    assert_eq!(fixture.machine.claude().calls(), [fixture.add()]);
    // Quoted as the platform's shell needs it: a Windows path has
    // backslashes, and the runner's temp folder a `~`.
    assert_eq!(
        registration.ran,
        [format!(
            "claude mcp add --scope user --transport stdio ffm-mcp -- {}",
            crate::console::shell_word(fixture.desired.command.as_str())
        )]
    );
    assert_eq!(
        registration.config,
        Some(fixture.machine.home().join(".claude.json"))
    );
}

#[test]
fn a_different_entry_is_removed_then_added() {
    let fixture = fixture();
    fixture.write_entry("/old/ffm-mcp");
    let registration = fixture.register();
    assert_eq!(registration.outcome, Outcome::Updated);
    assert_eq!(
        fixture.machine.claude().calls(),
        [fixture.remove(), fixture.add_json(&fixture.replaced())]
    );
    assert_eq!(registration.ran.len(), 2);
    let text = fs::read_to_string(fixture.machine.home().join(".claude.json")).unwrap();
    assert!(
        text.contains("oauthAccount"),
        "the sign-in survives: {text}"
    );
}

#[test]
fn a_registered_entry_runs_nothing() {
    let fixture = fixture();
    fixture.write_entry(fixture.desired.command.as_str());
    assert_eq!(fixture.register().outcome, Outcome::Unchanged);
    assert!(fixture.machine.claude().calls().is_empty());
}

#[test]
fn a_config_that_does_not_parse_is_never_handed_to_the_cli() {
    let fixture = fixture();
    fs::write(fixture.machine.home().join(".claude.json"), "{ broken").unwrap();
    let registration = fixture.register();
    assert!(matches!(
        registration.outcome,
        Outcome::Failed(Failure::Blocked(Blocker::Unparsable(_)))
    ));
    assert!(fixture.machine.claude().calls().is_empty());
}

#[test]
fn no_home_folder_stops_before_the_cli() {
    let mut fixture = fixture();
    fixture.host.home = None;
    let registration = fixture.register();
    assert_eq!(
        registration.outcome,
        Outcome::Failed(Failure::Blocked(Blocker::NoHomeFolder))
    );
    assert_eq!(registration.config, None);
}

#[test]
fn a_cli_that_left_path_is_missing() {
    let fixture = fixture();
    fs::remove_file(fixture.machine.bin().join("claude")).unwrap();
    assert_eq!(
        fixture.register().outcome,
        Outcome::Failed(Failure::CliMissing)
    );
}

#[test]
fn a_cli_that_vanishes_while_running_is_missing() {
    let fixture = fixture();
    fixture.machine.claude().behave(ClaudeBehaviour::Vanish);
    assert_eq!(
        fixture.register().outcome,
        Outcome::Failed(Failure::CliMissing)
    );
}

#[test]
fn a_cli_that_times_out_says_what_to_run_by_hand() {
    let fixture = fixture();
    fixture.machine.claude().behave(ClaudeBehaviour::TimeOut);
    let Outcome::Failed(failure) = fixture.register().outcome else {
        panic!("a timeout must fail");
    };
    assert!(matches!(failure, Failure::CliTimedOut { .. }));
    let text = failure.describe("x");
    assert!(
        text.starts_with("claude did not finish within 30 s. Run by hand: claude mcp add"),
        "{text}"
    );
}

#[test]
fn a_non_zero_exit_reports_the_last_stderr_line() {
    let fixture = fixture();
    fixture.machine.claude().behave(ClaudeBehaviour::Fail(
        "warming up\nError: not signed in".to_owned(),
    ));
    let registration = fixture.register();
    let Outcome::Failed(Failure::CliFailed { exit, message, .. }) = registration.outcome else {
        panic!("a failed exit must fail");
    };
    assert_eq!(exit, Some(1));
    assert_eq!(message, "Error: not signed in");
}

/// A runner that always answers the same.
struct Answer(Captured);

impl mcp_clients::Runner for Answer {
    fn run(&self, _call: &CliCall) -> io::Result<Captured> {
        Ok(self.0.clone())
    }
}

#[test]
fn a_silent_failure_falls_back_to_stdout_then_to_a_sentence() {
    let silent = Captured {
        exit: Exit::Failure(None),
        stdout: "\n".to_owned(),
        stderr: String::new(),
    };
    let chatty = Captured {
        stdout: "usage: claude\n".to_owned(),
        ..silent.clone()
    };
    let fixture = fixture();
    for (captured, expected) in [
        (silent, "claude failed without saying why"),
        (chatty, "usage: claude"),
    ] {
        let mut host = fixture.host.clone();
        host.runner = std::sync::Arc::new(Answer(captured));
        let Outcome::Failed(Failure::CliFailed { message, exit, .. }) =
            register(&host, &fixture.desired).outcome
        else {
            panic!("must fail");
        };
        assert_eq!(message, expected);
        assert_eq!(exit, None);
    }
}

#[test]
fn a_runner_error_is_a_cli_failure_with_its_message() {
    let fixture = fixture();
    let mut host = fixture.host.clone();
    host.runner = Host::default().runner;
    let Outcome::Failed(Failure::CliFailed { exit, message, .. }) =
        register(&host, &fixture.desired).outcome
    else {
        panic!("an unsupported runner must fail");
    };
    assert_eq!(exit, None);
    assert!(message.contains("no program runner"), "{message}");
}

#[test]
fn a_remove_that_fails_stops_before_the_add() {
    let fixture = fixture();
    fixture.write_entry("/old/ffm-mcp");
    fixture
        .machine
        .claude()
        .behave(ClaudeBehaviour::Fail("locked".to_owned()));
    let registration = fixture.register();
    let Outcome::Failed(failure) = &registration.outcome else {
        panic!("must fail: {:?}", registration.outcome);
    };
    assert!(matches!(failure, Failure::CliFailed { .. }), "{failure:?}");
    assert_eq!(fixture.machine.claude().calls(), [fixture.remove()]);
    // Review finding: the advice named only the add, which then fails with
    // "already exists". Both steps are still to run.
    let advice = failure.describe("x");
    assert!(
        advice.contains(&format!(
            "Run by hand: claude mcp remove --scope user ffm-mcp && claude mcp add-json --scope user ffm-mcp {}",
            crate::console::shell_word(&fixture.replaced().to_string())
        )),
        "{advice}"
    );
}

#[test]
fn an_add_that_fails_after_the_remove_says_the_entry_is_gone() {
    let fixture = fixture();
    fixture.write_entry("/old/ffm-mcp");
    fixture
        .machine
        .claude()
        .behave(ClaudeBehaviour::FailAdd("disk full".to_owned()));
    let Outcome::Failed(failure) = fixture.register().outcome else {
        panic!("must fail");
    };
    let Failure::RemovedNotAdded { message, command } = &failure else {
        panic!("expected RemovedNotAdded, got {failure:?}");
    };
    assert_eq!(message, "disk full (exit 1)");
    assert!(
        command.starts_with("claude mcp add-json --scope user ffm-mcp "),
        "{command}"
    );
    assert!(failure
        .describe("x")
        .starts_with("the old entry was removed"));
}

#[test]
fn an_exit_zero_without_a_write_is_not_verified() {
    let fixture = fixture();
    fixture.machine.claude().behave(ClaudeBehaviour::Pretend);
    assert_eq!(
        fixture.register().outcome,
        Outcome::Failed(Failure::NotVerified)
    );
}

#[test]
fn a_server_path_with_spaces_is_quoted_for_the_person_reading_it() {
    let fixture = fixture();
    let call = CliCall {
        args: vec!["mcp".to_owned(), "/srv/x/my tools/ffm-mcp".to_owned()],
        ..fixture.add()
    };
    let quoted = if cfg!(windows) {
        "\"/srv/x/my tools/ffm-mcp\""
    } else {
        "'/srv/x/my tools/ffm-mcp'"
    };
    assert_eq!(command_line(&call), format!("claude mcp {quoted}"));
}

#[test]
fn the_cli_is_never_used_for_the_current_folder() {
    let fixture = fixture();
    let folder = ProjectDir::new(fixture.machine.folder()).unwrap();
    let target = survey::Target::new(Scope::CurrentFolder, Some(&folder));
    let registration = super::super::register_client(
        &fixture.host,
        ClientId::ClaudeCode,
        target,
        &fixture.desired.command,
    );
    assert_eq!(registration.outcome, Outcome::Added);
    assert!(fixture.machine.claude().calls().is_empty());
    assert!(fixture.machine.folder().join(".mcp.json").is_file());
}

/// Review finding: replacing an entry with `remove` and `add` kept only the
/// command, so the owner's `env` and other keys were lost. `add-json` is
/// handed the old entry with this server set.
#[test]
fn a_replaced_entry_keeps_its_env_and_other_keys() {
    let fixture = fixture();
    let config = fixture.machine.home().join(".claude.json");
    let before = serde_json::json!({"oauthAccount": {"id": "x"}, "mcpServers": {"ffm-mcp": {
        "type": "stdio", "command": "/old/ffm-mcp", "args": ["-v"], "timeout": 5000,
        "env": {"FFM_MCP_TOKEN": "secret", "FFM_MCP_ROOT": "/srv/x/other"}
    }}});
    fs::write(&config, before.to_string()).unwrap();
    assert_eq!(fixture.register().outcome, Outcome::Updated);
    let after: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(
        after["mcpServers"]["ffm-mcp"],
        serde_json::json!({
            "type": "stdio", "command": fixture.desired.command.as_str(), "args": [],
            "timeout": 5000, "env": {"FFM_MCP_TOKEN": "secret"}
        })
    );
    assert_eq!(after["oauthAccount"]["id"], "x");
}

/// Review finding: "not verified" after a remove that worked did not say
/// the old entry was already gone.
#[test]
fn an_unverified_replace_says_the_old_entry_is_gone() {
    let fixture = fixture();
    fixture.write_entry("/old/ffm-mcp");
    fixture.machine.claude().behave(ClaudeBehaviour::Pretend);
    let Outcome::Failed(Failure::RemovedNotAdded { message, command }) = fixture.register().outcome
    else {
        panic!("an unverified replace leaves no entry");
    };
    assert!(
        message.starts_with("claude reported success but "),
        "{message}"
    );
    assert!(
        command.starts_with("claude mcp add-json --scope user ffm-mcp "),
        "{command}"
    );
}

/// A runner that fails every call with one kind of error.
struct Refuses(io::ErrorKind);

impl mcp_clients::Runner for Refuses {
    fn run(&self, _call: &CliCall) -> io::Result<Captured> {
        Err(io::Error::new(
            self.0,
            "cannot create a temporary file in /gone: No such file or directory",
        ))
    }
}

/// Review finding: `NotFound` from a missing working or temp folder was
/// reported as "claude CLI not found" while `claude` sat on PATH.
#[test]
fn not_found_while_the_program_is_there_is_a_cli_failure() {
    let fixture = fixture();
    let mut host = fixture.host.clone();
    host.runner = std::sync::Arc::new(Refuses(io::ErrorKind::NotFound));
    let Outcome::Failed(Failure::CliFailed { exit, message, .. }) =
        register(&host, &fixture.desired).outcome
    else {
        panic!("must be a CLI failure, not a missing CLI");
    };
    assert_eq!(exit, None);
    assert!(message.contains("temporary file"), "{message}");
}

#[test]
fn a_home_folder_that_is_gone_stops_before_the_cli() {
    let mut fixture = fixture();
    fixture.host.home = Some(fixture.machine.home().join("gone"));
    assert_eq!(
        fixture.register().outcome,
        Outcome::Failed(Failure::Blocked(Blocker::NoHomeFolder))
    );
    assert!(fixture.machine.claude().calls().is_empty());
}

/// Review finding: under `CLAUDE_CONFIG_DIR` the CLI writes another file
/// than the one FFM read and checked; no test followed the CLI there.
#[test]
fn claude_config_dir_is_where_the_entry_is_added_and_checked() {
    let mut fixture = fixture();
    let dir = fixture.machine.home().join("claude-dir");
    fs::create_dir(&dir).unwrap();
    fixture
        .host
        .env
        .insert("CLAUDE_CONFIG_DIR", dir.clone().into());
    fixture
        .machine
        .claude()
        .use_config(dir.join(".claude.json"));
    let registration = fixture.register();
    assert_eq!(registration.outcome, Outcome::Added);
    assert_eq!(registration.config, Some(dir.join(".claude.json")));
    assert_eq!(fixture.machine.claude().calls()[0].cwd, dir);
    assert!(!fixture.machine.home().join(".claude.json").exists());
}

#[test]
fn a_legacy_config_is_where_the_entry_is_added_and_checked() {
    let fixture = fixture();
    let legacy = fixture.machine.home().join(".claude").join(".config.json");
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::write(&legacy, "{}").unwrap();
    fixture.machine.claude().use_config(legacy.clone());
    let registration = fixture.register();
    assert_eq!(registration.outcome, Outcome::Added);
    assert_eq!(registration.config, Some(legacy));
}

#[test]
fn a_user_config_that_cannot_be_read_is_never_handed_to_the_cli() {
    let fixture = fixture();
    fs::create_dir(fixture.machine.home().join(".claude.json")).unwrap();
    assert_eq!(
        fixture.register().outcome,
        Outcome::Failed(Failure::Blocked(Blocker::Unreadable(
            "not a regular file".to_owned()
        )))
    );
    assert!(fixture.machine.claude().calls().is_empty());
}
