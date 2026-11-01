//! Purpose: Spawn only the administrator-configured FFM binary in headless mode.

use super::{connection::Connection, Error, Profile, Size};
use std::ffi::OsString;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

pub(super) trait Launcher: Send + Sync {
    fn launch(&self, root: &Path, size: Size, profile: Profile) -> Result<Connection, Error>;
}

pub(super) struct BinaryLauncher(pub(super) OsString);

impl Launcher for BinaryLauncher {
    fn launch(&self, root: &Path, size: Size, profile: Profile) -> Result<Connection, Error> {
        let mut child = command(&self.0, root, size, profile).spawn()?;
        let input = child
            .stdin
            .take()
            .ok_or(Error::Protocol("missing private FFM stdin"))?;
        let output = child
            .stdout
            .take()
            .ok_or(Error::Protocol("missing private FFM stdout"))?;
        let mut connection = Connection::from_io(output, input);
        connection.child = Some(child);
        Ok(connection)
    }
}

pub(super) fn command(binary: &OsString, root: &Path, size: Size, profile: Profile) -> Command {
    let mut command = Command::new(binary);
    command.args(["--automation", "--root"]).arg(root).args([
        "--width",
        &size.width.to_string(),
        "--height",
        &size.height.to_string(),
    ]);
    if matches!(profile, Profile::Isolated) {
        command.arg("--isolated");
    }
    command
        .current_dir(root)
        .env("FFM_LOG", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    command
}
