//! Purpose: Stage remote saves and publish successful versions without losing later edits.

use crate::models::remote_work::{FileLocation, RemoteTask};
use crate::models::{AfterSave, App, EditState, Editor, Fingerprint, RemotePath};
use crate::remote::{RemoteError, RemoteFs, RemoteMetadata, WriteMode};

pub(in crate::ops::remote) fn save(
    source: &FileLocation,
    expected: Fingerprint,
    bytes: &[u8],
    overwrite: bool,
) -> Result<(), RemoteError> {
    let FileLocation::Remote { link, path } = source else {
        return Err(RemoteError::Failed("no remote editor source"));
    };
    let facts = link.lock().stat(path)?;
    if facts.kind.is_symlink() || facts.kind.is_dir() {
        return Err(RemoteError::Unsupported(
            "saving a symbolic link or directory",
        ));
    }
    if !overwrite && Fingerprint::of(&super::super::read::bytes(source)?) != expected {
        return Err(RemoteError::Conflict);
    }
    let temporary = temporary_path(path)?;
    let mut fs = link.lock();
    let mut input = bytes;
    fs.upload(&temporary, &mut input, WriteMode::CreateNew)?;
    let result = commit(fs.as_mut(), &temporary, path, &facts);
    if result.is_err() {
        drop(fs.remove_file(&temporary));
    } // Cleanup never replaces the save's original error.
    result
}

fn commit(
    fs: &mut dyn RemoteFs,
    temporary: &RemotePath,
    path: &RemotePath,
    facts: &RemoteMetadata,
) -> Result<(), RemoteError> {
    if fs.stat(path)? != *facts {
        return Err(RemoteError::Conflict);
    }
    if let Some(mode) = facts.mode {
        fs.chmod(temporary, mode)?;
    }
    fs.replace_file(temporary, path)
}

fn temporary_path(path: &RemotePath) -> Result<RemotePath, RemoteError> {
    let unique = tempfile::tempdir()?;
    let leaf = unique
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(RemoteError::Failed(
            "cannot generate a temporary upload name",
        ))?;
    path.parent()
        .and_then(|parent| parent.join(&format!(".ffm-upload-{leaf}")))
        .ok_or(RemoteError::InvalidInput("invalid remote path"))
}

pub(in crate::ops::remote) fn saved(app: &mut App, task: &RemoteTask) {
    let RemoteTask::Save {
        source,
        bytes,
        then,
        revision,
        ..
    } = task
    else {
        return;
    };
    let Some(editor) = app.get_editor_mut().filter(|editor| {
        editor
            .remote
            .as_ref()
            .is_some_and(|remote| remote.location == *source)
    }) else {
        return;
    };
    let unchanged = editor.lsp.revision.0 == *revision;
    update_saved(editor, bytes, unchanged);
    if *then == AfterSave::Close && unchanged {
        app.close_overlay();
    }
}

fn update_saved(editor: &mut Editor, bytes: &[u8], unchanged: bool) {
    editor.notice = Some("Saved to remote server".to_owned());
    if let Some(remote) = &mut editor.remote {
        remote.fingerprint = Fingerprint::of(bytes);
        if let Some(name) = editor.path.file_name() {
            if std::fs::write(remote.temporary.path().join(name), bytes).is_err() {
                editor.notice = Some(
                    "Remote file saved; temporary working copy could not be refreshed".to_owned(),
                );
            }
        }
    }
    if unchanged {
        editor.state = EditState::Clean;
    }
    editor.disk = Some(Fingerprint::of(bytes));
    editor.lsp.saved = editor.lsp.saved.saturating_add(1);
}

pub fn save_direct(editor: &mut Editor) -> std::io::Result<()> {
    let Some(remote) = &editor.remote else {
        return Ok(());
    };
    let bytes = editor.format.encoding.encode(&editor.content())?;
    save(&remote.location, remote.fingerprint, &bytes, false).map_err(std::io::Error::other)?;
    update_saved(editor, &bytes, true);
    Ok(())
}
