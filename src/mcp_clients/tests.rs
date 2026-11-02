//! Unit tests for [`super`]: the client table, the entry comparison and the
//! words the dialog and `:mcp` share.

use std::path::PathBuf;

use super::*;

mod dialog;
mod entries;
mod labels;
mod paths;
mod table;

/// `unix` as an absolute path on this platform: Windows needs a drive.
pub(super) fn absolute(unix: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("C:{}", unix.replace('/', "\\")))
    } else {
        PathBuf::from(unix)
    }
}

pub(super) fn server() -> ServerBinary {
    ServerBinary::new(absolute("/srv/x/bin/ffm-mcp")).unwrap()
}

pub(super) fn folder() -> ProjectDir {
    ProjectDir::new(absolute("/srv/x/project")).unwrap()
}

pub(super) fn all_projects() -> ServerEntry {
    ServerEntry {
        command: server(),
        root: None,
    }
}

pub(super) fn current_folder() -> ServerEntry {
    ServerEntry {
        command: server(),
        root: Some(folder()),
    }
}
