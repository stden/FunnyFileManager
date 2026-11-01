//! Purpose: Own each captured process tree and stop its descendants together.

use std::{
    io,
    process::{Child, Command},
};

#[cfg(unix)]
pub(super) struct Tree;

#[cfg(windows)]
pub(super) struct Tree(Option<std::os::windows::io::OwnedHandle>);

impl Tree {
    #[cfg(unix)]
    pub(super) const fn of(_child: &Child) -> Self {
        Self
    }

    #[cfg(windows)]
    pub(super) fn of(child: &Child) -> Self {
        Self(crate::windows::process_tree_job(child).ok())
    }

    #[cfg(unix)]
    #[expect(
        clippy::unused_self,
        reason = "Windows owns a job; Unix uses the child's process group"
    )]
    pub(super) fn kill(self, child: &mut Child) -> io::Result<()> {
        crate::platform::kill_process_group(child).or_else(|_| child.kill())
    }

    #[cfg(windows)]
    pub(super) fn kill(self, child: &mut Child) -> io::Result<()> {
        drop(self.0);
        child.kill()
    }
}

#[cfg(windows)]
pub(super) fn detach(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000);
}

#[cfg(unix)]
pub(super) fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}
