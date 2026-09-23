//! Unit tests for [`super`]: the report's lines and the status line, for runs
//! that all worked, partly failed and wholly failed.

use crate::mcp_clients::ProjectDir;

use super::*;

fn server() -> ServerBinary {
    ServerBinary::new(PathBuf::from("/home/bob/.cargo/bin/ffm-mcp")).unwrap()
}

fn done(client: ClientId, outcome: Outcome) -> Registration {
    Registration {
        client,
        config: Some(PathBuf::from(format!("/home/bob/{}.json", client.name()))),
        ran: Vec::new(),
        outcome,
    }
}

#[test]
fn an_all_success_run_counts_what_changed() {
    let registrations = [
        Registration {
            ran: vec!["claude mcp add --scope user --transport stdio ffm-mcp -- x".to_owned()],
            ..done(ClientId::ClaudeCode, Outcome::Added)
        },
        done(ClientId::Codex, Outcome::Updated),
        done(ClientId::Kiro, Outcome::Unchanged),
    ];
    assert_eq!(
        status(&registrations),
        Status::CommandOutput("ffm-mcp: 1 added, 1 updated, 1 unchanged".to_owned())
    );
    let lines = lines(&server(), Target::AllProjects, &registrations);
    assert_eq!(lines[0], "ffm-mcp: /home/bob/.cargo/bin/ffm-mcp");
    assert_eq!(lines[1], "Scope: all projects (user level)");
    assert_eq!(
        lines[3],
        "Claude Code  added      ran: claude mcp add --scope user --transport stdio ffm-mcp -- x"
    );
    assert_eq!(lines[4], "Codex        updated    /home/bob/Codex.json");
    assert_eq!(lines[5], "Kiro         unchanged  /home/bob/Kiro.json");
    assert!(lines.contains(&"Notes:".to_owned()));
    assert!(lines.contains(
        &"  Kiro: may start ffm-mcp outside the project; prefer Current folder.".to_owned()
    ));
    assert_eq!(
        lines.last().unwrap(),
        "  Restart running clients, or reload their MCP servers, to pick up the change."
    );
}

#[test]
fn a_mixed_run_names_the_clients_that_failed() {
    let registrations = [
        done(ClientId::Codex, Outcome::Added),
        done(
            ClientId::Junie,
            Outcome::Failed(Failure::Blocked(Blocker::Unparsable(
                "expected value at line 3 column 1".to_owned(),
            ))),
        ),
    ];
    assert_eq!(
        status(&registrations),
        Status::Error("ffm-mcp: Junie failed; see the report".to_owned())
    );
    let lines = lines(&server(), Target::AllProjects, &registrations);
    assert_eq!(
        lines[4],
        "Junie        failed     /home/bob/Junie.json does not parse (expected value at line 3 \
         column 1); left untouched. Fix or remove it, then register again."
    );
}

#[test]
fn an_all_failed_run_names_every_client() {
    let registrations = [
        Registration::stopped(
            ClientId::ClaudeCode,
            None,
            Failure::Blocked(Blocker::NoHomeFolder),
        ),
        done(ClientId::Kiro, Outcome::Failed(Failure::NotVerified)),
    ];
    assert_eq!(
        status(&registrations),
        Status::Error("ffm-mcp: Claude Code, Kiro failed; see the report".to_owned())
    );
    let lines = lines(&server(), Target::AllProjects, &registrations);
    assert!(
        lines[3].ends_with("there is no home folder to register in"),
        "{}",
        lines[3]
    );
    assert!(
        lines[4].ends_with("/home/bob/Kiro.json has no matching ffm-mcp entry"),
        "{}",
        lines[4]
    );
}

#[test]
fn the_current_folder_is_named_with_its_notes() {
    let path = if cfg!(windows) {
        r"C:\srv\x\project"
    } else {
        "/srv/x/project"
    };
    let folder = ProjectDir::new(PathBuf::from(path)).unwrap();
    let registrations = [done(ClientId::Codex, Outcome::Added)];
    let lines = lines(&server(), Target::CurrentFolder(&folder), &registrations);
    assert_eq!(
        lines[1],
        format!("Scope: current folder (project level): {path}")
    );
    assert!(
        lines.contains(&"  Codex: reads .codex/config.toml only in folders it trusts.".to_owned())
    );
}

#[test]
fn file_and_cli_failures_read_as_sentences() {
    let io = Failure::Io {
        path: PathBuf::from("/home/bob/.kiro/settings/mcp.json"),
        message: "read-only file system".to_owned(),
    };
    assert_eq!(
        io.describe("the config"),
        "cannot write /home/bob/.kiro/settings/mcp.json: read-only file system"
    );
    assert_eq!(Failure::CliMissing.describe("x"), "claude CLI not found");
}

#[test]
fn cli_failures_say_what_to_run_by_hand() {
    let failed = |exit, message: &str| Failure::CliFailed {
        exit,
        message: message.to_owned(),
        command: "claude mcp add x".to_owned(),
    };
    let removed = Failure::RemovedNotAdded {
        message: "boom".to_owned(),
        command: "claude mcp add x".to_owned(),
    };
    let cases = [
        (
            failed(Some(2), "Error: bad"),
            "Error: bad (exit 2). Run by hand: claude mcp add x",
        ),
        (
            failed(None, "cannot run"),
            "cannot run. Run by hand: claude mcp add x",
        ),
        (
            removed,
            "the old entry was removed but the new one was not added: boom. \
             Run by hand: claude mcp add x",
        ),
    ];
    for (failure, expected) in cases {
        assert_eq!(failure.describe("the config"), expected);
    }
}

#[test]
fn outcome_words() {
    assert_eq!(Outcome::Added.word(), "added");
    assert_eq!(Outcome::Updated.word(), "updated");
    assert_eq!(Outcome::Unchanged.word(), "unchanged");
    assert_eq!(Outcome::Failed(Failure::CliMissing).word(), "failed");
    assert_eq!(REPORT_TITLE, "MCP server registration");
}
