//! Purpose: A renice planned before anything is sent — `]` and `[` one step
//! from each target's own value, `F7` one value for them all — with the
//! targets it leaves out and why, so the effect in `ops` only applies it.
//!
//! Planning is pure: the values come from the snapshot, and the refusals (a
//! bound, a Realtime class, a priority nobody could read) are decided here,
//! before any system call.

use super::targets::{first_refusal, FrozenTargets, Operation, Skipped, Target};
use super::{ControlRefusal, NiceStep, Niceness, Priority, PriorityChange, TaskList};

/// What a renice asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Renice {
    /// `]` or `[`: one step from each target's own value.
    Step(NiceStep),
    /// The Priority form's Set: this value for every target.
    Set(Niceness),
}

/// One target's part in a renice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedChange {
    target: Target,
    change: PriorityChange,
    /// Which way the priority moves. Raising it takes `CAP_SYS_NICE`, so a
    /// refusal is worded by it.
    direction: NiceStep,
}

impl PlannedChange {
    #[must_use]
    pub const fn target(&self) -> &Target {
        &self.target
    }

    #[must_use]
    pub const fn change(&self) -> PriorityChange {
        self.change
    }

    #[must_use]
    pub const fn direction(&self) -> NiceStep {
        self.direction
    }
}

/// A renice of frozen targets: the change each one gets, in view order, and
/// the targets left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NicePlan {
    renice: Renice,
    changes: Vec<PlannedChange>,
    skipped: Vec<Skipped>,
}

impl NicePlan {
    #[must_use]
    pub const fn renice(&self) -> Renice {
        self.renice
    }

    #[must_use]
    pub fn changes(&self) -> &[PlannedChange] {
        &self.changes
    }

    #[must_use]
    pub fn skipped(&self) -> &[Skipped] {
        &self.skipped
    }

    /// Why the plan changes nothing: `None` while a change remains.
    #[must_use]
    pub fn refusal(&self) -> Option<ControlRefusal> {
        self.changes
            .is_empty()
            .then(|| first_refusal(&self.skipped))
    }
}

impl TaskList {
    /// `]` or `[` for the marked processes, or else the cursor's: each one
    /// step from its own value, and refused at a bound.
    #[must_use]
    pub fn plan_step(&self, step: NiceStep) -> NicePlan {
        let frozen = self.freeze_targets(Operation::Renice);
        let mut plan = NicePlan {
            renice: Renice::Step(step),
            changes: Vec::new(),
            skipped: frozen.skipped().to_vec(),
        };
        for target in frozen.targets() {
            let target = target.clone();
            match self.priority_of(&target).stepped(step) {
                Ok(change) => plan.changes.push(PlannedChange {
                    target,
                    change,
                    direction: step,
                }),
                Err(refusal) => plan.skipped.push(Skipped::new(target, refusal)),
            }
        }
        plan
    }

    /// The Priority form's Set: `nice` for the `targets` it froze.
    #[must_use]
    pub fn plan_set(&self, targets: &FrozenTargets, nice: Niceness) -> NicePlan {
        let changes = targets.targets().iter().map(|target| PlannedChange {
            target: target.clone(),
            change: PriorityChange::Nice(nice),
            direction: direction(self.priority_of(target), nice),
        });
        NicePlan {
            renice: Renice::Set(nice),
            changes: changes.collect(),
            skipped: targets.skipped().to_vec(),
        }
    }

    /// The nice value the Priority form offers for `targets`: the first
    /// one's, or the normal 0 when it has none.
    #[must_use]
    pub fn offered_nice(&self, targets: &FrozenTargets) -> Niceness {
        let first = targets.targets().first();
        match first.map(|target| self.priority_of(target)) {
            Some(Priority::Unix { nice, .. }) => nice,
            Some(Priority::Windows { .. } | Priority::Unknown) | None => Niceness::NORMAL,
        }
    }

    /// `target`'s priority as the snapshot has it; unknown once its row is
    /// gone.
    fn priority_of(&self, target: &Target) -> Priority {
        self.row(target.key())
            .map_or(Priority::Unknown, |row| row.sample().priority)
    }
}

/// Which way setting `nice` moves a process now at `priority`: a lower nice
/// value raises it. Anything but a known higher nice value counts as a lower
/// priority, which only another user's ownership can refuse.
fn direction(priority: Priority, nice: Niceness) -> NiceStep {
    match priority {
        Priority::Unix { nice: now, .. } if nice < now => NiceStep::Raise,
        Priority::Unix { .. } | Priority::Windows { .. } | Priority::Unknown => NiceStep::Lower,
    }
}
