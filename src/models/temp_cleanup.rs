//! Purpose: Typed preview, confirmation and worker states for temporary-file cleanup.
use super::{Confirm, FileSize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};

#[derive(Clone, Default)]
pub enum TempSource {
    #[default]
    Unavailable,
    System,
    Directory {
        path: PathBuf,
        home: Option<PathBuf>,
    },
}
#[derive(Clone, Debug)]
pub struct TempRoot {
    pub(crate) path: PathBuf,
    pub(crate) identity: FileIdentity,
}
impl TempRoot {
    pub fn path(&self) -> &Path {
        &self.path
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileIdentity(pub(crate) file_id::FileId);
#[derive(Clone, Copy, Debug)]
pub struct TempCutoff(pub(crate) SystemTime);
impl TempCutoff {
    pub const RETENTION: Duration = Duration::from_hours(168);
    pub(crate) fn at(now: SystemTime) -> Result<Self, TempError> {
        now.checked_sub(Self::RETENTION)
            .map(Self)
            .ok_or(TempError::Clock)
    }
}
#[derive(Clone, Debug)]
pub struct PlannedTempFile {
    pub(crate) path: PathBuf,
    pub(crate) identity: FileIdentity,
    pub(crate) size: FileSize,
    pub(crate) modified: SystemTime,
}
#[derive(Clone, Debug)]
pub struct TempPlan {
    pub(crate) root: TempRoot,
    pub(crate) cutoff: TempCutoff,
    pub(crate) files: Vec<PlannedTempFile>,
    pub(crate) bytes: FileSize,
    pub(crate) skipped: usize,
}
#[derive(Clone, Copy, Default, Debug)]
pub struct TempOutcome {
    pub(crate) removed: usize,
    pub(crate) skipped: usize,
    pub(crate) failures: usize,
    pub(crate) freed: FileSize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TempError {
    UnsafeRoot,
    Unavailable,
    Scan,
    Clock,
    Limit,
    Worker,
}
impl TempError {
    pub fn message(self) -> &'static str {
        match self {
            Self::UnsafeRoot => "Unsafe temporary directory; cleanup refused",
            Self::Unavailable => "Temporary-file cleanup is unavailable in this session",
            Self::Scan => "Cannot scan the temporary directory",
            Self::Clock => "Cannot determine the temporary-file age",
            Self::Limit => "Temporary-file scan exceeded its safety limit",
            Self::Worker => "Temporary-file cleanup worker failed",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TempChoice {
    Clean,
    Cancel,
}
#[derive(Clone)]
pub enum TempCleanupState {
    QueuedScan(TempSource),
    Scanning,
    Preview { plan: TempPlan, focus: TempChoice },
    Confirm { plan: TempPlan, answer: Confirm },
    QueuedDelete(TempPlan),
    Deleting,
    Finished(TempOutcome),
    Failed(TempError),
}
#[derive(Clone)]
pub struct TempCleanupDialog {
    pub(crate) generation: Arc<()>,
    pub(crate) state: TempCleanupState,
}
impl TempCleanupDialog {
    pub(crate) fn new(source: TempSource) -> Self {
        Self {
            generation: Arc::new(()),
            state: TempCleanupState::QueuedScan(source),
        }
    }
}
