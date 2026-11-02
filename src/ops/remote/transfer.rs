//! Purpose: Local/remote tree transfers preserve the source until the complete copy succeeds.

use crate::models::remote_work::FileLocation;
use crate::remote::RemoteError;
use std::fs;
mod file;
mod location;
use file::copy_file;
use location::children;
pub(super) use location::{chmod, metadata, mkdir};

pub(super) fn transfer(
    source: &FileLocation,
    destination: &FileLocation,
    moving: bool,
) -> Result<(), RemoteError> {
    reject_inside(source, destination)?;
    if moving {
        if let (
            FileLocation::Remote {
                link: source_link,
                path: source_path,
            },
            FileLocation::Remote {
                link: destination_link,
                path: destination_path,
            },
        ) = (source, destination)
        {
            if source_link == destination_link {
                return source_link.lock().rename(source_path, destination_path);
            }
        }
    }
    copy_tree(source, destination)?;
    if moving {
        remove(source)?;
    }
    Ok(())
}

fn copy_tree(source: &FileLocation, destination: &FileLocation) -> Result<(), RemoteError> {
    let facts = metadata(source)?;
    if facts.kind.is_symlink() {
        return Err(RemoteError::Unsupported(
            "copying symbolic links between hosts",
        ));
    }
    if facts.kind.is_dir() {
        mkdir(destination)?;
        for name in children(source)? {
            copy_tree(&source.child(&name)?, &destination.child(&name)?)?;
        }
    } else {
        copy_file(source, destination)?;
    }
    if let Some(mode) = facts.mode {
        chmod(destination, mode)?;
    }
    Ok(())
}

pub(super) fn remove(location: &FileLocation) -> Result<(), RemoteError> {
    location.name()?; // Refuse filesystem roots before any recursive work.
    let facts = metadata(location)?;
    if facts.kind.is_dir() && !facts.kind.is_symlink() {
        for name in children(location)? {
            remove(&location.child(&name)?)?;
        }
    }
    match location {
        FileLocation::Local(path) if facts.kind.is_dir() && !facts.kind.is_symlink() => {
            fs::remove_dir(path)?;
        }
        FileLocation::Local(path) => fs::remove_file(path)?,
        FileLocation::Remote { link, path } if facts.kind.is_dir() && !facts.kind.is_symlink() => {
            link.lock().rmdir(path)?;
        }
        FileLocation::Remote { link, path } => link.lock().remove_file(path)?,
    }
    Ok(())
}

fn reject_inside(source: &FileLocation, destination: &FileLocation) -> Result<(), RemoteError> {
    let inside = match (source, destination) {
        (
            FileLocation::Remote {
                link: left,
                path: from,
            },
            FileLocation::Remote {
                link: right,
                path: to,
            },
        ) => left == right && (from == to || to.as_str().starts_with(&format!("{from}/"))),
        (FileLocation::Local(from), FileLocation::Local(to)) => to.starts_with(from),
        _ => false,
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
