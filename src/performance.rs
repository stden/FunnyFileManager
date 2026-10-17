//! Payload-free action timings, ranked by the time a user spent waiting.

use serde::Serialize;
use std::{collections::BTreeMap, io, path::Path, time::Duration};

#[derive(Debug, Clone, Default)]
pub struct Performance {
    actions: BTreeMap<String, Totals>,
}

#[derive(Debug, Clone, Default)]
struct Totals {
    /// Saturates at `u32::MAX`: four billion waits outlast any session, and a
    /// `u32` converts to `f64` exactly for the mean.
    count: u32,
    total: Duration,
    maximum: Duration,
    render: Duration,
}

#[derive(Debug, Serialize)]
struct Row<'a> {
    action: &'a str,
    count: u32,
    total_ms: f64,
    mean_ms: f64,
    max_ms: f64,
    render_ms: f64,
}

impl Performance {
    pub(crate) fn record(&mut self, action: &str, work: Duration, render: Duration) {
        let total = work.saturating_add(render);
        let entry = self.actions.entry(action.to_owned()).or_default();
        entry.count = entry.count.saturating_add(1);
        entry.total = entry.total.saturating_add(total);
        entry.render = entry.render.saturating_add(render);
        entry.maximum = entry.maximum.max(total);
    }

    fn rows(&self) -> Vec<Row<'_>> {
        let mut rows: Vec<_> = self
            .actions
            .iter()
            .map(|(action, stats)| Row {
                action,
                count: stats.count,
                total_ms: millis(stats.total),
                mean_ms: millis(stats.total) / f64::from(stats.count),
                max_ms: millis(stats.maximum),
                render_ms: millis(stats.render),
            })
            .collect();
        rows.sort_by(|a, b| {
            b.total_ms
                .total_cmp(&a.total_ms)
                .then(a.action.cmp(b.action))
        });
        rows
    }

    /// Current-session timings, most accumulated waiting first; all times are ms.
    pub fn report(&self) -> Vec<String> {
        let mut lines = vec![
            "Action statistics - current session".to_owned(),
            "Sorted by total waiting time; input idle time is excluded.".to_owned(),
            "Count    Total ms    Mean ms     Max ms  Render ms  Action".to_owned(),
        ];
        lines.extend(self.rows().iter().map(|row| {
            format!(
                "{:>5} {:>11.3} {:>10.3} {:>10.3} {:>10.3}  {}",
                row.count, row.total_ms, row.mean_ms, row.max_ms, row.render_ms, row.action,
            )
        }));
        if self.actions.is_empty() {
            lines.push("No actions recorded yet.".to_owned());
        }
        lines
    }

    /// Atomically save aggregates. Typed text, passwords, commands and paths are absent.
    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            crate::file_write::create_dirs_owner_only(parent)?;
        }
        let report = serde_json::json!({"version": 1, "actions": self.rows()});
        let text = serde_json::to_string_pretty(&report).map_err(io::Error::other)?;
        crate::file_write::write_owner_only(path, &text)
    }

    pub(crate) fn save_session(&self) -> io::Result<()> {
        self.save_session_in(
            crate::config::dir().as_deref(),
            std::time::SystemTime::now(),
        )
    }

    fn save_session_in(
        &self,
        directory: Option<&Path>,
        now: std::time::SystemTime,
    ) -> io::Result<()> {
        let directory = directory
            .ok_or_else(|| io::Error::other("no configuration directory for action statistics"))?;
        let timestamp = now
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        self.save_to(
            &directory
                .join("performance")
                .join(format!("session-{timestamp}-{}.json", std::process::id())),
        )
    }
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests;
