//! Purpose: Freeze the targets of a kill or a renice as the dialog opens: the
//! marked shown processes in view order, or else the cursor's process, each
//! with its relation to FFM and any refusal.

use super::super::tree::ancestors;
use super::super::{Pid, ProcessRow, TaskList};
use super::{refusal, FrozenTargets, Operation, Relation, Skipped, Target};

/// The rows `chosen` of `rows`, in that order, frozen for `operation`.
fn freeze(
    rows: &[ProcessRow],
    chosen: &[usize],
    own: Option<Pid>,
    operation: Operation,
) -> FrozenTargets {
    let above = own.map(|own| ancestors(rows, own)).unwrap_or_default();
    let mut frozen = FrozenTargets::default();
    for (row, process) in chosen
        .iter()
        .filter_map(|&row| rows.get(row).map(|process| (row, process)))
    {
        let sample = process.sample();
        let relation = if own == Some(sample.key.pid()) {
            Relation::Itself
        } else if above.contains(&row) {
            Relation::Ancestor
        } else {
            Relation::Other
        };
        let target = Target {
            key: sample.key,
            name: sample.name.clone(),
            owner: sample.owner.clone(),
            relation,
        };
        match refusal(sample, own, operation) {
            Some(refusal) => frozen.skipped.push(Skipped::new(target, refusal)),
            None => frozen.targets.push(target),
        }
    }
    frozen
}

impl TaskList {
    /// The targets of `operation` as they stand now: the marked shown
    /// processes in view order, or else the cursor's process.
    #[must_use]
    pub fn freeze_targets(&self, operation: Operation) -> FrozenTargets {
        let marked: Vec<usize> = self
            .view
            .iter()
            .filter(|shown| self.is_marked(shown.key()))
            .map(|shown| shown.row())
            .collect();
        let chosen = if marked.is_empty() {
            let cursor = self
                .cursor()
                .and_then(|cursor| self.view.get(cursor.index()));
            cursor.map(|shown| shown.row()).into_iter().collect()
        } else {
            marked
        };
        freeze(self.rows(), &chosen, self.own, operation)
    }
}
