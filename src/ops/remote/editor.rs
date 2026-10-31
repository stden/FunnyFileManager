//! Purpose: Remote editor buffers save through staged uploads and detect external changes.

use super::{context, enqueue};
use crate::models::remote_work::{FileLocation, RemoteTask};
use crate::models::{AfterSave, App, Fingerprint, Overlay, RemoteDocument, Viewer};
use crate::remote::RemoteError;
use std::{path::PathBuf, sync::Arc};

pub(super) fn open(
    app: &mut App,
    source: FileLocation,
    bytes: &[u8],
    edit: bool,
) -> Result<(), RemoteError> {
    if !edit {
        app.set_overlay(Overlay::Viewer(Viewer::file(
            PathBuf::from(source.label()),
            bytes,
        )));
        return Ok(());
    }
    let temporary = Arc::new(tempfile::tempdir()?);
    let path = temporary.path().join(source.name()?);
    std::fs::write(&path, bytes)?;
    let mut editor = crate::editor::editor_for(app, path, bytes)?;
    editor.title = source.label();
    editor.remote = Some(Box::new(RemoteDocument {
        location: source,
        fingerprint: Fingerprint::of(bytes),
        temporary,
    }));
    app.set_overlay(Overlay::Editor(editor));
    Ok(())
}

pub fn request_save(app: &mut App, then: AfterSave, overwrite: bool) -> bool {
    let Some(editor) = app.get_editor() else {
        return false;
    };
    let Some(remote) = &editor.remote else {
        return false;
    };
    let encoded = editor.format.encoding.encode(&editor.content());
    let task = encoded.map(|bytes| RemoteTask::Save {
        source: remote.location.clone(),
        bytes,
        expected: remote.fingerprint,
        overwrite,
        then,
        revision: editor.lsp.revision.0,
    });
    match task
        .map_err(RemoteError::from)
        .and_then(|task| Ok((context::operation_target(app)?, task)))
    {
        Ok((target, task)) => enqueue(app, target, task),
        Err(error) => {
            if let Some(editor) = app.get_editor_mut() {
                editor.notice = Some(error.to_string());
            }
        }
    }
    true
}

mod persist;
pub use persist::save_direct;
pub(super) use persist::{save, saved};
