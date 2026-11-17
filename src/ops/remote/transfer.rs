//! Purpose: Local/remote tree transfers preserve the source until the complete copy succeeds.

use crate::models::remote_work::FileLocation;
use crate::models::RemotePath;
use crate::remote::{RemoteError, RemoteLink, RemoteMetadata};
use std::fs;
mod file;
mod location;
use file::copy_file;
use location::{children, set_created_mode};
pub(super) use location::{chmod, metadata, mkdir};

pub(super) fn transfer(
    source: &FileLocation,
    destination: &FileLocation,
    moving: bool,
) -> Result<(), RemoteError> {
    reject_inside(source, destination)?;
    if moving {
        if let Some((link, from, to)) = same_remote_host(source, destination) {
            return link.lock().rename(from, to);
        }
    }
    copy_tree(source, destination)?;
    if moving {
        remove(source)?;
    }
    Ok(())
}

fn copy_tree(source: &FileLocation, destination: &FileLocation) -> Result<(), RemoteError> {
    copy_tree_with_facts(source, destination, &metadata(source)?)
}

fn copy_tree_with_facts(
    source: &FileLocation,
    destination: &FileLocation,
    facts: &RemoteMetadata,
) -> Result<(), RemoteError> {
    if facts.kind.is_symlink() {
        return Err(RemoteError::Unsupported(
            "copying symbolic links between hosts",
        ));
    }
    if facts.kind.is_dir() {
        mkdir(destination)?;
        for (name, child_facts) in children(source)? {
            let src_child = source.child(&name)?;
            let dst_child = destination.child(&name)?;
            copy_tree_with_facts(&src_child, &dst_child, &child_facts)?;
        }
    } else {
        copy_file(source, destination)?;
    }
    if let Some(mode) = facts.mode {
        set_created_mode(destination, mode)?;
    }
    Ok(())
}

pub(super) fn remove(location: &FileLocation) -> Result<(), RemoteError> {
    location.name()?; // Refuse filesystem roots before any recursive work.
    remove_with_facts(location, &metadata(location)?)
}

fn is_real_dir(facts: &RemoteMetadata) -> bool {
    facts.kind.is_dir() && !facts.kind.is_symlink()
}

fn remove_with_facts(location: &FileLocation, facts: &RemoteMetadata) -> Result<(), RemoteError> {
    let real_dir = is_real_dir(facts);
    if real_dir {
        for (name, child_facts) in children(location)? {
            remove_with_facts(&location.child(&name)?, &child_facts)?;
        }
    }
    match location {
        FileLocation::Local(path) if real_dir => fs::remove_dir(path)?,
        FileLocation::Local(path) => fs::remove_file(path)?,
        FileLocation::Remote { link, path } if real_dir => link.lock().rmdir(path)?,
        FileLocation::Remote { link, path } => link.lock().remove_file(path)?,
    }
    Ok(())
}

fn same_remote_host<'a>(
    source: &'a FileLocation,
    destination: &'a FileLocation,
) -> Option<(&'a RemoteLink, &'a RemotePath, &'a RemotePath)> {
    match (source, destination) {
        (
            FileLocation::Remote {
                link: source_link,
                path: source_path,
            },
            FileLocation::Remote {
                link: destination_link,
                path: destination_path,
            },
        ) if source_link == destination_link => Some((source_link, source_path, destination_path)),
        _ => None,
    }
}

fn is_same_or_descendant_remote(from: &RemotePath, to: &RemotePath) -> bool {
    to.as_str()
        .strip_prefix(from.as_str())
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

fn reject_inside(source: &FileLocation, destination: &FileLocation) -> Result<(), RemoteError> {
    let inside = if let Some((_, from, to)) = same_remote_host(source, destination) {
        is_same_or_descendant_remote(from, to)
    } else if let (FileLocation::Local(from), FileLocation::Local(to)) = (source, destination) {
        to.starts_with(from)
    } else {
        false
    };
    if inside {
        return Err(RemoteError::InvalidInput(
            "cannot copy or move an item into itself",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
