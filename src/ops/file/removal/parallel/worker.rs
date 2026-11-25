//! Purpose: Worker-thread execution for parallel directory scanning, batched
//! file unlinking, and empty-directory removal.

use std::path::{Path, PathBuf};
use std::sync::{atomic::Ordering, PoisonError};
use std::{fs, io};

#[cfg(any(test, tarpaulin))]
use super::super::RemovalStep;
use super::{Coordinator, Task, BATCH_SIZE};
use crate::utils::FailedAt;

type EntryItem = (PathBuf, fs::FileType);
type DirItem = (usize, PathBuf);

fn inspect_entry(
    dir: &Path,
    entry: io::Result<fs::DirEntry>,
    #[cfg(any(test, tarpaulin))] forced: Option<RemovalStep>,
) -> io::Result<(PathBuf, fs::FileType)> {
    #[cfg(any(test, tarpaulin))]
    super::super::forced_with(forced, RemovalStep::NextEntry).map_err(FailedAt::for_path(dir))?;
    let entry = entry.map_err(FailedAt::for_path(dir))?;
    let path = entry.path();
    #[cfg(any(test, tarpaulin))]
    super::super::forced_with(forced, RemovalStep::EntryKind).map_err(FailedAt::for_path(&path))?;
    let file_type = entry.file_type().map_err(FailedAt::for_path(&path))?;
    Ok((path, file_type))
}

impl Coordinator {
    pub(super) fn worker_loop(&self) {
        while let Some(task) = self.next_task() {
            if let Err(error) = self.run_task(task) {
                self.fail(error);
            }
            self.finish_task();
        }
    }

    pub(super) fn fail(&self, error: io::Error) {
        {
            let mut queue = self.lock();
            self.stop.store(true, Ordering::Release);
            if queue.error.is_none() {
                queue.error = Some(error);
            }
        }
        self.work_cv.notify_all();
        self.main_cv.notify_one();
    }

    fn next_task(&self) -> Option<Task> {
        let mut queue = self.lock();
        loop {
            if self.is_stopped() {
                return None;
            }
            if let Some(task) = queue.tasks.pop() {
                queue.active += 1;
                return Some(task);
            }
            queue = self
                .work_cv
                .wait(queue)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    fn finish_task(&self) {
        let wake_main = {
            let mut queue = self.lock();
            queue.active -= 1;
            self.is_stopped() || (queue.active == 0 && queue.tasks.is_empty())
        };
        if wake_main {
            self.main_cv.notify_one();
        }
    }

    fn run_task(&self, task: Task) -> io::Result<()> {
        match task {
            Task::ScanDir { path, depth } => self.scan_dir(&path, depth),
            Task::UnlinkFiles(files) => self.unlink_files(files),
            Task::RemoveDirs(dirs) => self.remove_dirs(dirs),
        }
    }

    pub(super) fn scan_dir(&self, dir: &Path, depth: usize) -> io::Result<()> {
        #[cfg(any(test, tarpaulin))]
        super::super::forced_with(self.forced, RemovalStep::ListFolder)
            .map_err(FailedAt::for_path(dir))?;
        let entries = fs::read_dir(dir).map_err(FailedAt::for_path(dir))?;
        let mut subdirs = Vec::new();
        let mut batch = Vec::with_capacity(BATCH_SIZE);
        for entry in entries.take_while(|_| !self.is_stopped()) {
            let (path, file_type) = inspect_entry(
                dir,
                entry,
                #[cfg(any(test, tarpaulin))]
                self.forced,
            )?;
            self.collect_entry(depth, path, file_type, &mut subdirs, &mut batch);
        }
        self.flush_scan(subdirs, batch);
        Ok(())
    }

    pub(super) fn collect_entry(
        &self,
        depth: usize,
        path: PathBuf,
        file_type: fs::FileType,
        subdirs: &mut Vec<DirItem>,
        batch: &mut Vec<EntryItem>,
    ) {
        if file_type.is_dir() {
            subdirs.push((depth + 1, path));
            if subdirs.len() >= BATCH_SIZE {
                self.push_subdirs(std::mem::take(subdirs));
            }
            return;
        }
        batch.push((path, file_type));
        if batch.len() >= BATCH_SIZE {
            self.push_unlink_batch(std::mem::take(batch));
        }
    }

    pub(super) fn flush_scan(&self, subdirs: Vec<DirItem>, batch: Vec<EntryItem>) {
        if !subdirs.is_empty() {
            self.push_subdirs(subdirs);
        }
        if !batch.is_empty() {
            self.push_unlink_batch(batch);
        }
    }

    fn push_subdirs(&self, subdirs: Vec<DirItem>) {
        let mut queue = self.lock();
        for (depth, path) in subdirs {
            queue.dirs.push((depth, path.clone()));
            queue.tasks.push(Task::ScanDir { path, depth });
        }
        drop(queue);
        self.work_cv.notify_all();
    }

    fn push_unlink_batch(&self, batch: Vec<EntryItem>) {
        let count = u64::try_from(batch.len()).unwrap_or(u64::MAX);
        let sample = batch.first().map(|(path, _)| path.clone());
        let mut queue = self.lock();
        queue.discovered = queue.discovered.saturating_add(count);
        if queue.current.is_none() {
            queue.current = sample;
        }
        queue.tasks.push(Task::UnlinkFiles(batch));
        drop(queue);
        self.work_cv.notify_one();
        self.main_cv.notify_one();
    }

    pub(super) fn unlink_files(&self, files: Vec<EntryItem>) -> io::Result<()> {
        let mut count = 0_u64;
        let mut last = None;
        for (path, file_type) in files.into_iter().take_while(|_| !self.is_stopped()) {
            super::super::remove_link_or_file_with(
                &path,
                file_type,
                #[cfg(any(test, tarpaulin))]
                self.forced,
            )
            .map_err(FailedAt::for_path(&path))?;
            count += 1;
            last = Some(path);
        }
        let mut queue = self.lock();
        queue.removed = queue.removed.saturating_add(count);
        if last.is_some() {
            queue.current = last;
        }
        drop(queue);
        self.main_cv.notify_one();
        Ok(())
    }

    pub(super) fn remove_dirs(&self, dirs: Vec<PathBuf>) -> io::Result<()> {
        for path in dirs.into_iter().take_while(|_| !self.is_stopped()) {
            super::super::remove_empty_dir(
                &path,
                #[cfg(any(test, tarpaulin))]
                self.forced,
            )
            .map_err(FailedAt::for_path(&path))?;
        }
        Ok(())
    }
}
