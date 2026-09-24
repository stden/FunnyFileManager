//! Purpose: The task list's process information dialog (F3, Enter): what
//! the snapshot holds about one process, labelled, the command line wrapped
//! to four rows. No file is read for it.

use ratatui::Frame;

use crate::models::{MemoryUsage, ProcessKey, ProcessRow, ProcessState, TaskList};
use crate::mouse::DialogMouse;

use super::super::super::dialog::{render_tracked, Button, Ground, Row};
use super::super::super::panel::{clip_name, pad_cells};
use super::super::super::{printable_name, take_cells, Cells};
use super::super::format;

/// Where a label's value starts.
const LABEL: Cells = Cells::new(14);
/// The command line's width, per row.
const COMMAND_WRAP: Cells = Cells::new(50);
/// The command line's rows, at most.
const COMMAND_ROWS: usize = 4;

/// What the snapshot holds about one process; no file is read for it.
pub(super) fn info(f: &mut Frame<'_>, tracker: &mut DialogMouse, list: &TaskList, key: ProcessKey) {
    let lines = list.row(key).map_or_else(
        || vec![format!("Process {} has exited.", key.pid())],
        info_lines,
    );
    let rows: Vec<Row<'_>> = lines.iter().map(|line| Row::Plain(line)).collect();
    let buttons = [Button::new("OK").default_button().focused(true)];
    render_tracked(
        f,
        tracker,
        "Process information",
        Ground::Prompt,
        &rows,
        &buttons,
    );
}

fn info_lines(process: &ProcessRow) -> Vec<String> {
    let sample = process.sample();
    let (pri, nice) = format::priority(sample.priority);
    let parent = sample.parent.map(|pid| pid.to_string());
    let mut lines = vec![
        field("PID", &sample.key.pid().to_string()),
        field("Parent", parent.as_deref().unwrap_or(format::UNKNOWN)),
        field("Name", &printable_name(&sample.name)),
    ];
    lines.extend(command_lines(&printable_name(&sample.command)));
    let running = sample.running_for.map(format::span);
    lines.extend([
        field("User", &format::owner(&sample.owner)),
        field("State", &state(sample.state)),
        field("Priority/Nice", &format!("{pri} / {nice}")),
        field("Threads", &sample.threads.to_string()),
        field("Running for", running.as_deref().unwrap_or(format::UNKNOWN)),
        field("CPU time", &format::cpu_time(sample.cpu_time)),
        field("VIRT/RES/SHR", &memory(sample.memory)),
    ]);
    lines
}

/// `label` padded to the value column, then `value`.
fn field(label: &str, value: &str) -> String {
    format!("{}{value}", pad_cells(label, LABEL))
}

/// The command line in rows of [`COMMAND_WRAP`] cells, at most
/// [`COMMAND_ROWS`], the last cut with Far's `}` when more is left.
fn command_lines(command: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut rest = command;
    for row in 0..COMMAND_ROWS {
        let label = if row == 0 { "Command" } else { "" };
        if row + 1 == COMMAND_ROWS {
            lines.push(field(label, &clip_name(rest, COMMAND_WRAP)));
            break;
        }
        let (head, _) = take_cells(rest, COMMAND_WRAP);
        lines.push(field(label, &head));
        rest = rest.strip_prefix(head.as_str()).unwrap_or_default();
        if rest.is_empty() {
            break;
        }
    }
    lines
}

/// The state letter and what it means: `R (running)`.
fn state(state: ProcessState) -> String {
    let meaning = match state {
        ProcessState::Running => "running",
        ProcessState::Sleeping => "sleeping",
        ProcessState::DiskSleep => "waiting for the disk",
        ProcessState::Zombie => "zombie",
        ProcessState::Stopped => "stopped",
        ProcessState::TracingStop => "stopped by a tracer",
        ProcessState::Dead => "dead",
        ProcessState::Idle => "idle",
        ProcessState::Parked => "parked",
        ProcessState::Other(_) => return state.letter().to_string(),
    };
    format!("{} ({meaning})", state.letter())
}

/// VIRT, RES and SHR, `-` for what the platform did not give.
fn memory(usage: Option<MemoryUsage>) -> String {
    let Some(usage) = usage else {
        return format!("{0} / {0} / {0}", format::UNKNOWN);
    };
    format!(
        "{} / {} / {}",
        format::bytes(usage.virtual_size),
        format::bytes(usage.resident),
        format::size(usage.shared)
    )
}
