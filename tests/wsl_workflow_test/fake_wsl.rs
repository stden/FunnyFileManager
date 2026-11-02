//! Purpose: A scripted WSL backend that records updates and can be made to stay busy.

use super::*;

pub struct FakeWsl {
    pub operations: Mutex<Vec<WslOperation>>,
    pub root: PathBuf,
    pub release_update: Option<Mutex<mpsc::Receiver<()>>>,
}

impl FakeWsl {
    pub fn new(root: PathBuf) -> Self {
        Self {
            operations: Mutex::new(Vec::new()),
            root,
            release_update: None,
        }
    }

    #[expect(
        clippy::expect_used,
        reason = "test fixture: the lock is poisoned only by a panic, which has already failed the test"
    )]
    pub fn operations(&self) -> Vec<WslOperation> {
        self.operations.lock().expect("recorded operations").clone()
    }
}

impl WslBackend for FakeWsl {
    fn execute(&self, operation: &WslOperation) -> Result<WslOutput, String> {
        self.operations
            .lock()
            .map_err(|error| format!("record operations: {error}"))?
            .push(operation.clone());
        match operation {
            WslOperation::Refresh => Ok(WslOutput::Distributions(vec![
                distribution("Ubuntu"),
                distribution("Debian"),
            ])),
            WslOperation::Update => {
                if let Some(release) = &self.release_update {
                    release
                        .lock()
                        .map_err(|error| format!("update release: {error}"))?
                        .recv_timeout(WAIT_LIMIT)
                        .map_err(|error| format!("test did not release the update: {error}"))?;
                }
                Ok(WslOutput::Updated(PLATFORM_DONE.to_owned()))
            }
            WslOperation::UpdatePackages(_) => {
                Ok(WslOutput::PackagesUpdated(PACKAGES_DONE.to_owned()))
            }
            WslOperation::Open(_) => Ok(WslOutput::Root(self.root.clone())),
        }
    }
}

#[expect(
    clippy::expect_used,
    reason = "test fixture: every name passed here is a valid literal distribution name"
)]
pub fn distribution(name: &str) -> DistributionName {
    DistributionName::new(name).expect("valid fixture distribution")
}
