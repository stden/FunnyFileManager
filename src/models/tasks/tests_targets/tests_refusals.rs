//! Purpose: unit tests for the refusals before any system call and the sentence
//! each one reads as.

use super::*;

fn unknown_start(raw: u32) -> ProcessSample {
    ProcessSample {
        key: ProcessKey::new(pid(raw), Start::Unknown),
        ..process(raw, "hidden")
    }
}

fn kernel_thread(raw: u32) -> ProcessSample {
    ProcessSample {
        kind: ProcessKind::KernelThread,
        ..process(raw, "kworker")
    }
}

#[test]
fn signals_are_refused_to_ffm_init_kernel_threads_and_the_unverifiable() {
    let cases = [
        (process(OWN, "ffm"), Some(ControlRefusal::OwnProcess)),
        (process(1, "systemd"), Some(ControlRefusal::Init)),
        (process(2, "bash"), None),
        (kernel_thread(3), Some(ControlRefusal::KernelThread)),
        (unknown_start(4), Some(ControlRefusal::Unverifiable)),
    ];
    for (sample, expected) in cases {
        let raw = sample.key.pid();
        assert_eq!(
            refusal(&sample, Some(pid(OWN)), Operation::Signal),
            expected,
            "{raw}"
        );
    }
}

#[test]
fn a_renice_is_refused_only_when_the_process_cannot_be_confirmed() {
    for sample in [process(OWN, "ffm"), process(1, "systemd"), kernel_thread(3)] {
        assert_eq!(refusal(&sample, Some(pid(OWN)), Operation::Renice), None);
    }
    let unknown = unknown_start(4);
    assert_eq!(
        refusal(&unknown, Some(pid(OWN)), Operation::Renice),
        Some(ControlRefusal::Unverifiable)
    );
}

#[test]
fn nothing_left_to_do_is_explained_by_the_first_refusal() {
    let empty = listing(Vec::new()).freeze_targets(Operation::Signal);
    assert_eq!(empty.refusal(), Some(ControlRefusal::NoTarget));
    let init = listing(vec![process(1, "init")]).freeze_targets(Operation::Signal);
    assert_eq!(init.refusal(), Some(ControlRefusal::Init));
    let vim = listing(vec![process(2, "vim")]).freeze_targets(Operation::Signal);
    assert_eq!(vim.refusal(), None);
}

#[test]
fn every_refusal_reads_as_one_sentence() {
    let nineteen = Niceness::MAX;
    let sentences = [
        (
            ControlRefusal::OwnProcess,
            "This is FFM itself; quit with F10.",
        ),
        (
            ControlRefusal::Init,
            "PID 1 (init) is never signalled from here.",
        ),
        (
            ControlRefusal::KernelThread,
            "Kernel threads ignore signals.",
        ),
        (
            ControlRefusal::Unverifiable,
            "FFM cannot confirm which process this is; nothing was sent.",
        ),
        (ControlRefusal::AtLimit(nineteen), "Nice is already 19."),
        (
            ControlRefusal::RealtimeClass,
            "The Realtime class is not changed from here.",
        ),
        (ControlRefusal::NoTarget, "No process is selected."),
    ];
    for (refusal, sentence) in sentences {
        assert_eq!(refusal.to_string(), sentence);
    }
}

/// The platform asks the PID rules before its first system call, and the
/// kind rule once it has re-read the process; together they are the rule
/// the dialog freezes targets by.
#[test]
fn the_signal_rules_split_into_one_by_pid_and_one_by_kind() {
    let own = Some(pid(OWN));
    let by_pid = |raw| ControlRefusal::signal_to_pid(pid(raw), own);
    assert_eq!(by_pid(OWN), Some(ControlRefusal::OwnProcess));
    assert_eq!(by_pid(1), Some(ControlRefusal::Init));
    assert_eq!(by_pid(2), None);
    assert_eq!(ControlRefusal::signal_to_pid(pid(OWN), None), None);
    let by_kind = ControlRefusal::signal_to_kind;
    assert_eq!(
        by_kind(ProcessKind::KernelThread),
        Some(ControlRefusal::KernelThread)
    );
    assert_eq!(by_kind(ProcessKind::User), None);
}
