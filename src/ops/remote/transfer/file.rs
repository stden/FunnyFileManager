//! Purpose: Stream local/remote file transfers directly, staging on disk only for host-to-host copies.

use crate::models::remote_work::FileLocation;
use crate::remote::{RemoteError, WriteMode};
use std::{
    fs,
    io::{BufReader, BufWriter, Seek, SeekFrom, Write},
    path::Path,
};

const STREAM_BUFFER_BYTES: usize = 256 * 1024;

pub(super) fn copy_file(
    source: &FileLocation,
    destination: &FileLocation,
) -> Result<(), RemoteError> {
    match (source, destination) {
        (FileLocation::Local(src), FileLocation::Remote { link, path }) => {
            let mut reader = open_buffered_local(src)?;
            link.lock()
                .upload(path, &mut reader, WriteMode::CreateNew)?;
            Ok(())
        }
        (FileLocation::Remote { link, path }, FileLocation::Local(dst)) => {
            download_to_local(link, path, dst)
        }
        (FileLocation::Local(src), FileLocation::Local(dst)) => copy_local_to_local(src, dst),
        (
            FileLocation::Remote {
                link: src_link,
                path: src_path,
            },
            FileLocation::Remote {
                link: dst_link,
                path: dst_path,
            },
        ) => copy_remote_to_remote(src_link, src_path, dst_link, dst_path),
    }
}

fn download_to_local(
    link: &crate::remote::RemoteLink,
    path: &crate::models::RemotePath,
    dst: &Path,
) -> Result<(), RemoteError> {
    write_created_local(dst, |output| {
        let mut writer = BufWriter::with_capacity(STREAM_BUFFER_BYTES, output);
        link.lock().download(path, &mut writer)?;
        Ok(writer.flush()?)
    })
}

fn copy_local_to_local(src: &Path, dst: &Path) -> Result<(), RemoteError> {
    let mut reader = open_buffered_local(src)?;
    write_created_local(dst, |output| {
        std::io::copy(&mut reader, output)?;
        Ok(())
    })
}

fn open_buffered_local(path: &Path) -> std::io::Result<BufReader<fs::File>> {
    let (file, meta) = crate::utils::open_regular_file_with_metadata(path)?;
    let cap = usize::try_from(meta.len())
        .unwrap_or(STREAM_BUFFER_BYTES)
        .clamp(8 * 1024, STREAM_BUFFER_BYTES);
    Ok(BufReader::with_capacity(cap, file))
}

fn copy_remote_to_remote(
    src_link: &crate::remote::RemoteLink,
    src_path: &crate::models::RemotePath,
    dst_link: &crate::remote::RemoteLink,
    dst_path: &crate::models::RemotePath,
) -> Result<(), RemoteError> {
    let mut staged = tempfile::tempfile()?;
    src_link.lock().download(src_path, &mut staged)?;
    staged.seek(SeekFrom::Start(0))?;
    dst_link
        .lock()
        .upload(dst_path, &mut staged, WriteMode::CreateNew)?;
    Ok(())
}

pub(in crate::ops::remote) fn write_created_local(
    path: &Path,
    write_into: impl FnOnce(&mut fs::File) -> Result<(), RemoteError>,
) -> Result<(), RemoteError> {
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    if let Err(error) = write_into(&mut output) {
        drop(output);
        drop(fs::remove_file(path)); // Only the file exclusively created above is cleaned up.
        return Err(error);
    }
    Ok(())
}
