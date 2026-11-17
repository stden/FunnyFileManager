//! Purpose: Unit tests for `OwnedChild` attachment errors, detach fallback, and drop error logging.

use super::*;
use portable_pty::{native_pty_system, ChildKiller, PtySize};

#[derive(Debug)]
struct FakeChild {
    fail_wait: bool,
}

impl ChildKiller for FakeChild {
    fn kill(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
        Box::new(Self {
            fail_wait: self.fail_wait,
        })
    }
}

impl Child for FakeChild {
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        Ok(None)
    }

    fn wait(&mut self) -> io::Result<ExitStatus> {
        if self.fail_wait {
            Err(io::Error::other("stop failed"))
        } else {
            Ok(ExitStatus::with_exit_code(0))
        }
    }

    fn process_id(&self) -> Option<u32> {
        None
    }

    #[cfg(windows)]
    fn as_raw_handle(&self) -> Option<std::os::windows::io::RawHandle> {
        None
    }
}

fn open_master() -> Box<dyn MasterPty + Send> {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("pty");
    pair.master
}

#[test]
fn new_rejects_handleless_child_and_drop_logs_failed_stop() {
    let rejected = OwnedChild::new(
        Box::new(FakeChild { fail_wait: false }),
        open_master(),
        Descendants::Owned,
    );
    assert!(rejected.is_err());

    let failing = OwnedChild {
        child: Box::new(FakeChild { fail_wait: true }),
        master: Master::new(open_master()),
        descendants: Descendants::Detached,
        lifetime: Lifetime::Running,
        #[cfg(windows)]
        tree: None,
    };
    #[cfg(windows)]
    failing.detach().expect("detach without job handle");
    drop(failing);
}

#[cfg(unix)]
#[test]
fn stop_group_ignores_esrch_for_non_leader_child() {
    let mut child = std::process::Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn()
        .expect("spawn");
    assert!(stop_group(&child).is_ok());
    drop(child.wait());
}

#[cfg(unix)]
fn refused(_child: &mut OwnedChild) -> io::Result<()> {
    Err(io::ErrorKind::PermissionDenied.into())
}

#[cfg(unix)]
#[test]
fn failed_drop_cleanup_reports_the_error_and_retains_the_child_for_retry() {
    let child = std::process::Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn()
        .expect("spawn");
    let mut owned =
        OwnedChild::new(Box::new(child), open_master(), Descendants::Owned).expect("owned");
    let mut errors = Vec::new();
    owned.cleanup(refused, &mut |message| errors.push(message.to_owned()));
    assert_eq!(errors.len(), 1);
    assert!(errors[0].starts_with("Cannot stop private terminal child:"));
    assert!(matches!(owned.lifetime, Lifetime::Running));
    owned.stop().expect("retry the stop");
    assert!(matches!(owned.lifetime, Lifetime::Reaped));
}
