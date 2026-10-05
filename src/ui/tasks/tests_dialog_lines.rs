//! Unit tests for the lines the task list's dialogs write about their
//! targets: why each refused target was skipped, the Priority form over
//! several processes, and every process state in the information dialog.

use crossterm::event::KeyCode;

use super::tests::{draw, kthreadd, lines};
use crate::models::{
    KernelPriority, Priority, PriorityClass, ProcessKey, ProcessSample, ProcessState, Start,
};
use crate::prompt::Privilege;
use crate::test_support::task_list::{named, nice, pid, TaskApp, OWN};

/// Every row of `app` drawn at 80×24.
fn screen(app: &mut TaskApp) -> String {
    let terminal = draw(app, 80, 24);
    lines(terminal.backend().buffer()).join("\n")
}

/// `processes`, all of them marked.
fn all_marked(processes: Vec<ProcessSample>) -> TaskApp {
    let mut app = TaskApp::open(processes, Privilege::User);
    app.list().toggle_kernel_threads();
    app.list().invert_marks();
    app
}

#[test]
fn a_signal_names_why_each_refused_target_is_skipped() {
    let unknown = ProcessSample {
        key: ProcessKey::new(pid(77), Start::Unknown),
        ..named(77, "ghost")
    };
    let mut app = all_marked(vec![
        named(1, "systemd"),
        kthreadd(),
        unknown,
        named(OWN, "ffm"),
        named(50, "vim"),
    ]);
    app.press(KeyCode::F(8));
    let drawn = screen(&mut app);
    for line in [
        "Do you wish to terminate the process?",
        "50 vim",
        "Skipped: 1 systemd (init)",
        "Skipped: 2 kthreadd (kernel thread)",
        "Skipped: 77 ghost (identity unknown)",
        "Skipped: FFM itself",
    ] {
        assert!(drawn.contains(line), "{line:?} missing from\n{drawn}");
    }
}

#[test]
fn a_renice_names_why_each_refused_target_is_skipped() {
    let at_nineteen = ProcessSample {
        priority: Priority::Unix {
            kernel: KernelPriority::new(39),
            nice: nice(19),
        },
        ..named(20, "batch")
    };
    let unknown = ProcessSample {
        priority: Priority::Unknown,
        ..named(30, "odd")
    };
    let realtime = ProcessSample {
        priority: Priority::Windows {
            class: PriorityClass::Realtime,
        },
        ..named(40, "audio")
    };
    let mut app = all_marked(vec![at_nineteen, unknown, realtime, named(50, "vim")]);
    app.press(KeyCode::Char('['));
    let drawn = screen(&mut app);
    for line in [
        "Lower the priority of the process?",
        "50 vim",
        "Skipped: 20 batch (at its limit)",
        "Skipped: 30 odd (priority unknown)",
        "Skipped: 40 audio (realtime)",
    ] {
        assert!(drawn.contains(line), "{line:?} missing from\n{drawn}");
    }
}

#[test]
fn the_priority_form_over_several_processes_counts_them() {
    let mut app = all_marked(vec![named(10, "a"), named(20, "b"), named(30, "c")]);
    app.press(KeyCode::F(7));
    let drawn = screen(&mut app);
    assert!(drawn.contains("║3 processes "), "{drawn}");
}

#[test]
fn the_information_dialog_says_what_each_state_letter_means() {
    let states = [
        (ProcessState::Running, "R (running)"),
        (ProcessState::Sleeping, "S (sleeping)"),
        (ProcessState::DiskSleep, "D (waiting for the disk)"),
        (ProcessState::Zombie, "Z (zombie)"),
        (ProcessState::Stopped, "T (stopped)"),
        (ProcessState::TracingStop, "t (stopped by a tracer)"),
        (ProcessState::Dead, "X (dead)"),
        (ProcessState::Idle, "I (idle)"),
        (ProcessState::Parked, "P (parked)"),
        (ProcessState::from_letter(b'W'), "State         W "),
    ];
    for (state, shown) in states {
        let process = ProcessSample {
            state,
            ..named(10, "sh")
        };
        let mut app = TaskApp::open(vec![process], Privilege::User);
        app.press(KeyCode::Enter);
        let drawn = screen(&mut app);
        assert!(drawn.contains(shown), "{shown:?} missing from\n{drawn}");
    }
}
