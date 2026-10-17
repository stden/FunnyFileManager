//! Unit tests for [`super`]: added, updated and unchanged files, symlinks, new
//! folders, and the failures that must leave a file exactly as it was.

use crate::mcp_clients::{Blocker, ClientId, ProjectDir, Scope};
use crate::test_support::{file_identity, FakeMachine};

use super::super::survey::project_dir;
use super::*;

struct Fixture {
    machine: FakeMachine,
    server: ServerBinary,
    folder: ProjectDir,
}

fn fixture() -> Fixture {
    let machine = FakeMachine::new().expect("fake machine");
    let server = ServerBinary::new(machine.install_server().unwrap()).unwrap();
    let folder = project_dir(&machine.folder()).unwrap();
    Fixture {
        machine,
        server,
        folder,
    }
}

impl Fixture {
    fn register(&self, client: ClientId, target: Target<'_>) -> Registration {
        register(&self.machine.host(), client.client(), target, &self.server)
    }
}

/// The command the `ffm-mcp` entry of a JSON config starts, read as JSON:
/// the file escapes a Windows path's backslashes, so the path itself is not
/// in its text.
fn json_command(text: &str) -> Option<String> {
    let config: serde_json::Value = serde_json::from_str(text).ok()?;
    let command = config.pointer("/mcpServers/ffm-mcp/command")?.as_str()?;
    Some(command.to_owned())
}

mod write_failures;
mod writing;
