//! Unit tests for [`super`]: the exact `claude` calls, the check after them,
//! and every way the CLI can fail — against the emulator, never a real CLI.

use std::{fs, io};

use crate::mcp_clients::{Captured, Exit, ProjectDir, Scope, ServerBinary};
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

/// A runner that always answers the same.
struct Answer(Captured);

impl mcp_clients::Runner for Answer {
    fn run(&self, _call: &CliCall) -> io::Result<Captured> {
        Ok(self.0.clone())
    }
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
    assert_eq!(fixture.machine.claude().calls().len(), 0);
    assert!(fixture.machine.folder().join(".mcp.json").is_file());
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

mod cli_failures;
mod config_locations;
mod planning;
mod replace_steps;
