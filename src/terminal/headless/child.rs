//! Purpose: Own each private PTY's process tree until inherited handles close.

use super::master::Master;
use portable_pty::{Child, ExitStatus, MasterPty};
use std::io;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Descendants {
    Owned,
    Detached,
}

enum Lifetime {
    Running,
    Reaped,
}

pub(super) struct OwnedChild {
    child: Box<dyn Child + Send + Sync>,
    master: Master,
    descendants: Descendants,
    lifetime: Lifetime,
    #[cfg(windows)]
    tree: Option<std::os::windows::io::OwnedHandle>,
}

impl OwnedChild {
    pub fn new(
        child: Box<dyn Child + Send + Sync>,
        master: Box<dyn MasterPty + Send>,
        descendants: Descendants,
    ) -> io::Result<Self> {
        #[cfg(unix)]
        {
            let native: &dyn Child = child.as_ref();
            if native.downcast_ref::<std::process::Child>().is_none() {
                return Err(io::Error::other("Unsupported private terminal process"));
            }
        }
        #[cfg(windows)]
        let mut child = child;
        #[cfg(windows)]
        let tree = match attach(&*child) {
            Ok(tree) => Some(tree),
            Err(error) => {
                // The failed attachment still leaves this launch owning the child.
                drop(child.kill());
                drop(child.wait());
                return Err(error);
            }
        };
        Ok(Self {
            child,
            master: Master::new(master),
            descendants,
            lifetime: Lifetime::Running,
            #[cfg(windows)]
            tree,
        })
    }

    pub fn master(&self) -> &Master {
        &self.master
    }

    pub fn poll(&mut self) -> io::Result<Option<ExitStatus>> {
        let status = self.poll_owned()?;
        if status.is_some() {
            self.lifetime = Lifetime::Reaped;
        }
        Ok(status)
    }

    #[cfg(windows)]
    fn poll_owned(&mut self) -> io::Result<Option<ExitStatus>> {
        let status = self.child.try_wait()?;
        if let Some(status) = &status {
            if self.descendants == Descendants::Detached && status.success() {
                self.detach()?;
            }
            drop(self.tree.take());
        }
        Ok(status)
    }

    #[cfg(windows)]
    fn detach(&self) -> io::Result<()> {
        if let Some(tree) = &self.tree {
            crate::windows::disarm_kill_on_close(tree)?;
        }
        Ok(())
    }

    #[cfg(unix)]
    fn poll_owned(&mut self) -> io::Result<Option<ExitStatus>> {
        let native = self.native()?;
        if !crate::platform::has_exited(native)? {
            return Ok(None);
        }
        // Keep the zombie's group identity until descendants are stopped.
        if self.descendants == Descendants::Owned {
            self.master.foreground_stop()?;
            stop_group(native)?;
        }
        self.child.try_wait()
    }

    pub fn stop(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        drop(self.tree.take());
        #[cfg(unix)]
        {
            self.master.foreground_stop()?;
            stop_group(self.native()?)?;
        }
        self.child.wait()?;
        self.lifetime = Lifetime::Reaped;
        Ok(())
    }

    #[cfg(unix)]
    fn native(&self) -> io::Result<&std::process::Child> {
        let child: &dyn Child = self.child.as_ref();
        child
            .downcast_ref::<std::process::Child>()
            .ok_or_else(|| io::Error::other("Unsupported private terminal process"))
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if matches!(self.lifetime, Lifetime::Running) {
            if let Err(error) = self.stop() {
                crate::diagnostics::log_error(&format!(
                    "Cannot stop private terminal child: {error}"
                ));
            }
        }
    }
}

#[cfg(windows)]
fn attach(child: &dyn Child) -> io::Result<std::os::windows::io::OwnedHandle> {
    crate::windows::private_terminal_job(child)
}

#[cfg(unix)]
fn stop_group(child: &std::process::Child) -> io::Result<()> {
    match crate::platform::kill_process_group(child) {
        Err(error) if error.raw_os_error() == Some(3) => Ok(()), // No remaining group.
        result => result,
    }
}
