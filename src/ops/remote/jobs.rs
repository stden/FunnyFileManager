//! Purpose: Execute one queued remote job off the UI thread.

use super::{read, transfer};
use crate::models::remote_work::{
    FileLocation, RemoteListing, RemoteOutcome, RemoteRefresh, RemoteTask,
};
use crate::models::{RemotePath, Session};
use crate::remote::{Credentials, RemoteError, RemoteLink};

pub fn execute(task: &RemoteTask) -> Result<RemoteOutcome, RemoteError> {
    match task {
        RemoteTask::Connect {
            session,
            credentials,
        } => connect(session, credentials),
        RemoteTask::List { link, path } => list(link, path),
        RemoteTask::Open { source, edit } => Ok(RemoteOutcome::Opened {
            source: source.clone(),
            bytes: read::bytes(source)?,
            edit: *edit,
        }),
        RemoteTask::Refresh(listings) => Ok(refresh(listings)),
        RemoteTask::Save {
            source,
            expected,
            bytes,
            overwrite,
            ..
        } => {
            super::editor::save(source, *expected, bytes, *overwrite)?;
            Ok(RemoteOutcome::Saved)
        }
        mutation => {
            mutate(mutation)?;
            Ok(RemoteOutcome::Done)
        }
    }
}

fn connect(session: &Session, credentials: &Credentials) -> Result<RemoteOutcome, RemoteError> {
    let link = RemoteLink::new(crate::remote::connect_with(session, credentials)?);
    let path = session.remote_dir.clone().unwrap_or_else(RemotePath::root);
    let entries = link.lock().list(&path)?;
    Ok(RemoteOutcome::Listed {
        link,
        path,
        entries,
    })
}

fn list(link: &RemoteLink, path: &RemotePath) -> Result<RemoteOutcome, RemoteError> {
    let entries = link.lock().list(path)?;
    Ok(RemoteOutcome::Listed {
        link: link.clone(),
        path: path.clone(),
        entries,
    })
}

fn refresh(listings: &[RemoteListing]) -> RemoteOutcome {
    RemoteOutcome::Refreshed(
        listings
            .iter()
            .map(|listing| {
                let result = listing.link.lock().list(&listing.path);
                RemoteRefresh {
                    listing: listing.clone(),
                    result,
                }
            })
            .collect(),
    )
}

fn mutate(task: &RemoteTask) -> Result<(), RemoteError> {
    match task {
        RemoteTask::Transfer {
            sources,
            destination,
            moving,
        } => transfer_batch(sources, destination, *moving),
        RemoteTask::Delete(sources) => sources.iter().try_for_each(transfer::remove),
        RemoteTask::Mkdir(path) => transfer::mkdir(path),
        RemoteTask::Rename { from, to } => transfer::transfer(from, to, true),
        RemoteTask::Chmod { targets, mode } => targets
            .iter()
            .try_for_each(|target| transfer::chmod(target, *mode)),
        RemoteTask::Connect { .. }
        | RemoteTask::List { .. }
        | RemoteTask::Open { .. }
        | RemoteTask::Refresh(_)
        | RemoteTask::Save { .. } => Err(RemoteError::Failed("not a filesystem mutation")),
    }
}

fn transfer_batch(
    sources: &[FileLocation],
    destination: &FileLocation,
    moving: bool,
) -> Result<(), RemoteError> {
    for source in sources {
        transfer::transfer(source, &destination.child(source.name()?)?, moving)?;
    }
    Ok(())
}
