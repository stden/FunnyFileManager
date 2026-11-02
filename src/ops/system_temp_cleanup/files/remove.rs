//! Purpose: Revalidate and lock each planned file before unlinking that file only.
use super::{policy, scan::snapshot};
use crate::models::{
    temp_cleanup::{PlannedTempFile, TempOutcome, TempPlan},
    FileSize,
};
use std::{fs, io};
pub fn remove(plan: &TempPlan) -> TempOutcome {
    let mut outcome = TempOutcome {
        skipped: plan.skipped,
        ..TempOutcome::default()
    };
    for file in &plan.files {
        match remove_one(plan, file) {
            Ok(Some(bytes)) => {
                outcome.removed += 1;
                outcome.freed = FileSize::from_bytes(outcome.freed.bytes().saturating_add(bytes));
            }
            Ok(None) => outcome.skipped += 1,
            Err(_error) => outcome.failures += 1,
        }
    }
    outcome
}
fn remove_one(plan: &TempPlan, file: &PlannedTempFile) -> io::Result<Option<u64>> {
    if !unchanged(plan, file) {
        return Ok(None);
    }
    let handle = match open(&file.path) {
        Ok(handle) => handle,
        Err(_error) => return Ok(None),
    };
    if handle.try_lock().is_err() || !unchanged(plan, file) {
        return Ok(None);
    }
    let metadata = handle.metadata()?;
    if !policy::eligible(&metadata, plan.cutoff)
        || metadata.len() != file.size.bytes()
        || metadata.modified()? != file.modified
    {
        return Ok(None);
    }
    fs::remove_file(&file.path)?;
    Ok(Some(file.size.bytes()))
}
fn unchanged(plan: &TempPlan, file: &PlannedTempFile) -> bool {
    if !policy::safe_parent(&plan.root, &file.path) {
        return false;
    }
    let Ok(metadata) = fs::symlink_metadata(&file.path) else {
        return false;
    };
    if !policy::eligible(&metadata, plan.cutoff) {
        return false;
    }
    snapshot(&file.path, &metadata).is_ok_and(|current| {
        current.identity == file.identity
            && current.size == file.size
            && current.modified == file.modified
    })
}

#[cfg(windows)]
fn open(path: &std::path::Path) -> io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
    fs::File::options()
        .read(true)
        .write(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}
#[cfg(unix)]
fn open(path: &std::path::Path) -> io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    let flags = i32::try_from((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits())
        .map_err(io::Error::other)?;
    fs::File::options()
        .read(true)
        .write(true)
        .custom_flags(flags)
        .open(path)
}
#[cfg(not(any(windows, unix)))]
fn open(path: &std::path::Path) -> io::Result<fs::File> {
    fs::File::options().read(true).write(true).open(path)
}
