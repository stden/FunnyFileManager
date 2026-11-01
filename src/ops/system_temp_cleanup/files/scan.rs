//! Purpose: Build a bounded file-only plan without following links or altering temporary contents.
use super::policy;
use crate::models::{
    temp_cleanup::{PlannedTempFile, TempCutoff, TempError, TempPlan, TempSource},
    FileSize,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};
const MAX_ENTRIES: usize = 100_000;
pub fn scan(source: &TempSource, now: SystemTime) -> Result<TempPlan, TempError> {
    let mut plan = new_plan(source, now)?;
    let mut pending = vec![plan.root.path.clone()];
    let mut visited = 0;
    while let Some(directory) = pending.pop() {
        if !policy::safe_parent(&plan.root, &directory.join(".ffm-probe")) {
            plan.skipped += 1;
            continue;
        }
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_error) if directory == plan.root.path => return Err(TempError::Scan),
            Err(_error) => {
                plan.skipped += 1;
                continue;
            }
        };
        for entry in entries {
            visited += 1;
            if visited > MAX_ENTRIES {
                return Err(TempError::Limit);
            }
            inspect(entry, &mut plan, &mut pending)?;
        }
    }
    Ok(plan)
}
fn inspect(
    entry: std::io::Result<fs::DirEntry>,
    plan: &mut TempPlan,
    pending: &mut Vec<PathBuf>,
) -> Result<(), TempError> {
    let result = entry.and_then(|entry| Ok((entry.path(), entry.metadata()?)));
    let Ok((path, metadata)) = result else {
        plan.skipped += 1;
        return Ok(());
    };
    if !policy::on_root_device(&metadata, &plan.root) {
        plan.skipped += 1;
        return Ok(());
    }
    if metadata.is_dir() && !policy::linked(&metadata) && policy::owned(&metadata) {
        pending.push(path);
    } else if policy::eligible(&metadata, plan.cutoff) && policy::safe_parent(&plan.root, &path) {
        match snapshot(&path, &metadata) {
            Ok(file) => {
                let bytes = plan
                    .bytes
                    .bytes()
                    .checked_add(file.size.bytes())
                    .ok_or(TempError::Limit)?;
                plan.bytes = FileSize::from_bytes(bytes);
                plan.files.push(file);
            }
            Err(_error) => plan.skipped += 1,
        }
    } else {
        plan.skipped += 1;
    }
    Ok(())
}
pub(super) fn snapshot(path: &Path, metadata: &fs::Metadata) -> std::io::Result<PlannedTempFile> {
    Ok(PlannedTempFile {
        path: path.to_owned(),
        identity: policy::identity(path)?,
        size: FileSize::from_bytes(metadata.len()),
        modified: metadata.modified()?,
    })
}

fn new_plan(source: &TempSource, now: SystemTime) -> Result<TempPlan, TempError> {
    Ok(TempPlan {
        root: policy::root(source)?,
        cutoff: TempCutoff::at(now)?,
        files: Vec::new(),
        bytes: FileSize::from_bytes(0),
        skipped: 0,
    })
}
