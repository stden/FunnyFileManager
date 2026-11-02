//! Purpose: Bounded background pool for uncached child-repository `git status` probes.
//!
//! Directory loading records only cached or `IN_PROGRESS` repositories and
//! leaves cache misses on `PanelMetadata::dirty_probe`. This controller drains
//! those misses off the UI thread, cancels queued work when a panel navigates
//! elsewhere, and requests a frame only when a visible badge changes.

use std::path::{Path, PathBuf};
use std::sync::Arc;

mod pool;

use self::pool::{Pool, Prober};
use super::Redraw;
use crate::models::{App, FileName};

const MAX_WORKERS: usize = 6;

pub(super) struct GitDirtyJobs {
    prober: Prober,
    max_workers: usize,
    pool: Option<Pool>,
}

impl Default for GitDirtyJobs {
    fn default() -> Self {
        Self {
            prober: Arc::new(crate::git::probe_child_repo),
            max_workers: MAX_WORKERS,
            pool: None,
        }
    }
}

impl Drop for GitDirtyJobs {
    fn drop(&mut self) {
        if let Some(pool) = &self.pool {
            pool.close();
        }
    }
}

impl GitDirtyJobs {
    #[cfg(test)]
    pub(super) fn with_prober<F>(prober: F) -> Self
    where
        F: Fn(&Path, &FileName) -> bool + Send + Sync + 'static,
    {
        Self::with_workers(MAX_WORKERS, prober)
    }

    #[cfg(test)]
    pub(super) fn with_workers<F>(max_workers: usize, prober: F) -> Self
    where
        F: Fn(&Path, &FileName) -> bool + Send + Sync + 'static,
    {
        Self {
            prober: Arc::new(prober),
            max_workers: max_workers.max(1),
            pool: None,
        }
    }

    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        let left_batch = app.left_panel.take_dirty_probe();
        let right_batch = app.right_panel.take_dirty_probe();
        if self.pool.is_none() && left_batch.is_none() && right_batch.is_none() {
            return Redraw::Unchanged;
        }
        let active = [
            app.left_panel.path.as_path(),
            app.right_panel.path.as_path(),
        ];
        self.enqueue(active, left_batch, right_batch);
        self.collect(app)
    }

    fn enqueue(
        &mut self,
        active: [&Path; 2],
        left_batch: Option<(PathBuf, Vec<FileName>)>,
        right_batch: Option<(PathBuf, Vec<FileName>)>,
    ) {
        let max_workers = self.max_workers;
        let pool = self.pool.get_or_insert_with(Pool::new);
        let demand = pool.update_queue(active, [left_batch, right_batch]);
        pool.grow_to(demand.min(max_workers), &self.prober);
    }

    fn collect(&self, app: &mut App) -> Redraw {
        let Some(pool) = &self.pool else {
            return Redraw::Unchanged;
        };
        let mut redraw = Redraw::Unchanged;
        while let Ok((parent, name, dirty)) = pool.try_recv() {
            let left = app.left_panel.apply_dirty_probe(&parent, &name, dirty);
            let right = app.right_panel.apply_dirty_probe(&parent, &name, dirty);
            if left || right {
                redraw = Redraw::Needed;
            }
        }
        redraw
    }
}

#[cfg(test)]
mod tests;
