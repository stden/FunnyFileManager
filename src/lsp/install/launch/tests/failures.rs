//! Purpose: Java runtime probes that fail — cancellation and timeouts end the
//! search, and a runtime that cannot run is never offered as Java.

use super::*;

struct FailedRuntime {
    path: PathBuf,
    error: InstallError,
}

impl Environment for FailedRuntime {
    fn program(&self, _names: &[&str], _missing: &'static str) -> Result<PathBuf, InstallError> {
        Ok(self.path.clone())
    }

    fn run(&self, _command: &mut Command, _stop: &AtomicBool) -> Result<Vec<u8>, InstallError> {
        Err(self.error)
    }
}

#[test]
fn java_version_probes_preserve_cancellation_and_timeout() {
    let root = TempDir::new().unwrap();
    for error in [InstallError::Cancelled, InstallError::Timeout] {
        let environment = FailedRuntime {
            path: root.path().join("java"),
            error,
        };
        assert_eq!(java(&AtomicBool::new(false), &environment), Err(error));
    }
}

#[test]
fn a_runtime_that_cannot_execute_is_not_offered_as_java() {
    let root = TempDir::new().unwrap();
    let environment = FailedRuntime {
        path: root.path().join("java"),
        error: InstallError::Command,
    };
    assert!(matches!(
        java(&AtomicBool::new(false), &environment),
        Err(InstallError::Missing(_))
    ));
}
