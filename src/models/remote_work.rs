//! Purpose: Remote jobs and locations describe work without doing filesystem or network I/O.

use super::{DirEntry, PanelSide, RemotePath, Session, SessionName};
use crate::remote::{Credentials, RemoteError, RemoteLink, UnixMode};

mod location;
pub use location::FileLocation;

#[derive(Clone)]
pub enum RemoteTask {
    Connect {
        session: Session,
        credentials: Credentials,
    },
    List {
        link: RemoteLink,
        path: RemotePath,
    },
    Transfer {
        sources: Vec<FileLocation>,
        destination: FileLocation,
        moving: bool,
    },
    Delete(Vec<FileLocation>),
    Mkdir(FileLocation),
    Rename {
        from: FileLocation,
        to: FileLocation,
    },
    Chmod {
        targets: Vec<FileLocation>,
        mode: UnixMode,
    },
    Open {
        source: FileLocation,
        edit: bool,
    },
    Refresh(Vec<RemoteListing>),
    Save {
        source: FileLocation,
        bytes: Vec<u8>,
        expected: super::Fingerprint,
        overwrite: bool,
        then: super::AfterSave,
        revision: u64,
    },
}

#[derive(Clone)]
pub enum RemoteTarget {
    Connect {
        side: PanelSide,
        session: SessionName,
    },
    Panel {
        side: PanelSide,
        link: RemoteLink,
        path: RemotePath,
    },
}

#[derive(Clone)]
pub struct RemoteRequest {
    pub id: u64,
    pub target: RemoteTarget,
    pub task: RemoteTask,
}

#[derive(Clone, Default)]
enum WorkState {
    #[default]
    Idle,
    Queued(Box<RemoteRequest>),
    Running(u64),
}

#[derive(Clone, Default)]
pub struct RemoteRequests {
    next: u64,
    state: WorkState,
}

impl RemoteRequests {
    pub(crate) const fn is_idle(&self) -> bool {
        matches!(self.state, WorkState::Idle)
    }
    pub(crate) fn queue(
        &mut self,
        target: RemoteTarget,
        task: RemoteTask,
    ) -> Result<(), RemoteError> {
        if !matches!(self.state, WorkState::Idle) {
            return Err(RemoteError::Failed("a remote operation is already running"));
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or(RemoteError::Failed("remote request counter exhausted"))?;
        self.state = WorkState::Queued(Box::new(RemoteRequest {
            id: self.next,
            target,
            task,
        }));
        Ok(())
    }

    pub(crate) fn take(&mut self) -> Option<RemoteRequest> {
        let WorkState::Queued(_) = self.state else {
            return None;
        };
        let WorkState::Queued(request) =
            std::mem::replace(&mut self.state, WorkState::Running(self.next))
        else {
            return None;
        };
        Some(*request)
    }

    pub(crate) fn finish(&mut self, id: u64) -> bool {
        if matches!(self.state, WorkState::Running(running) if running == id) {
            self.state = WorkState::Idle;
            return true;
        }
        false
    }
}

pub enum RemoteOutcome {
    Listed {
        link: RemoteLink,
        path: RemotePath,
        entries: Vec<DirEntry>,
    },
    Done,
    Opened {
        source: FileLocation,
        bytes: Vec<u8>,
        edit: bool,
    },
    Refreshed(Vec<RemoteRefresh>),
    Saved,
}

#[derive(Clone)]
pub struct RemoteListing {
    pub side: PanelSide,
    pub link: RemoteLink,
    pub path: RemotePath,
}

pub struct RemoteRefresh {
    pub listing: RemoteListing,
    pub result: Result<Vec<DirEntry>, RemoteError>,
}
