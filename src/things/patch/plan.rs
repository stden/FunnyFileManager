//! Purpose: Rolling wave planner that separates HA sibling hosts into distinct waves.

use super::{PatchKind, PatchPlan, PatchScope, PatchTarget, PatchWave};
use crate::things::{Machine, Status};

/// Build an ordered patch plan from catalog machines with HA twin separation.
pub fn build_plan<'a>(
    machines: impl IntoIterator<Item = &'a Machine>,
    scope: PatchScope,
    reboot: bool,
    comment: &str,
) -> PatchPlan {
    let mut rhel = Vec::new();
    let mut windows = Vec::new();
    let mut skipped = Vec::new();
    for machine in machines {
        classify_target(machine, scope, &mut rhel, &mut windows, &mut skipped);
    }
    let waves = build_waves(rhel, windows);
    PatchPlan {
        waves,
        skipped,
        reboot,
        comment: comment.to_owned(),
    }
}

fn classify_target(
    machine: &Machine,
    scope: PatchScope,
    rhel: &mut Vec<PatchTarget>,
    windows: &mut Vec<PatchTarget>,
    skipped: &mut Vec<String>,
) {
    if machine.id.trim().is_empty() {
        skipped.push(format!("{}: missing id", machine.name));
        return;
    }
    if machine.status == Status::Offline {
        skipped.push(format!("{}: offline", machine.name));
        return;
    }
    let Some(kind) = PatchKind::classify(machine) else {
        skipped.push(format!("{}: unknown OS", machine.name));
        return;
    };
    if !scope.allows(kind) {
        skipped.push(format!("{}: excluded by scope", machine.name));
        return;
    }
    let target = PatchTarget {
        id: machine.id.clone(),
        name: machine.name.clone(),
        kind,
    };
    match kind {
        PatchKind::Rhel => rhel.push(target),
        PatchKind::Windows => windows.push(target),
    }
}

fn build_waves(rhel: Vec<PatchTarget>, windows: Vec<PatchTarget>) -> Vec<PatchWave> {
    let mut waves = Vec::new();
    push_os_waves(&mut waves, PatchKind::Rhel, rhel);
    push_os_waves(&mut waves, PatchKind::Windows, windows);
    for (idx, wave) in waves.iter_mut().enumerate() {
        wave.index = idx + 1;
    }
    waves
}

fn push_os_waves(waves: &mut Vec<PatchWave>, kind: PatchKind, targets: Vec<PatchTarget>) {
    if targets.is_empty() {
        return;
    }
    let (wave_one, wave_two) = split_ha_waves(targets);
    waves.push(PatchWave {
        index: 0,
        kind,
        targets: wave_one,
    });
    if !wave_two.is_empty() {
        waves.push(PatchWave {
            index: 0,
            kind,
            targets: wave_two,
        });
    }
}

fn split_ha_waves(mut targets: Vec<PatchTarget>) -> (Vec<PatchTarget>, Vec<PatchTarget>) {
    targets.sort_by_cached_key(|target| target.name.to_ascii_lowercase());
    let mut wave_one: Vec<PatchTarget> = Vec::new();
    let mut wave_two: Vec<PatchTarget> = Vec::new();
    for target in targets {
        let sibling_in_one = wave_one
            .last()
            .is_some_and(|prev| are_ha_twins(&prev.name, &target.name));
        if sibling_in_one {
            wave_two.push(target);
        } else {
            wave_one.push(target);
        }
    }
    (wave_one, wave_two)
}

fn are_ha_twins(first: &str, second: &str) -> bool {
    let Some((prefix_a, num_a)) = split_numeric_suffix(first) else {
        return false;
    };
    let Some((prefix_b, num_b)) = split_numeric_suffix(second) else {
        return false;
    };
    prefix_a.eq_ignore_ascii_case(prefix_b) && num_a.abs_diff(num_b) == 1
}

fn split_numeric_suffix(name: &str) -> Option<(&str, u64)> {
    let short = name.split('.').next().unwrap_or(name);
    let digit_start = short
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_ascii_digit())
        .map(|(idx, _)| idx)
        .last()?;
    let (prefix, digits) = short.split_at_checked(digit_start)?;
    if prefix.is_empty() {
        return None;
    }
    digits.parse::<u64>().ok().map(|num| (prefix, num))
}
