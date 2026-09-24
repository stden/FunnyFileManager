//! The fixture the task list's screen tests share: the machine and the
//! processes of the design's sketch, and drawing a list to rows of text.

mod machine;

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::style::Color;
use ratatui::Terminal;

use crate::models::{
    KernelPriority, MemoryBytes, MemoryUsage, Niceness, Owner, Priority, ProcessKind,
    ProcessSample, ProcessState, Sample,
};
use crate::prompt::Privilege;
use crate::test_support::processes::{process, sample};
use crate::test_support::task_list::{key_of, pid, TaskApp};
use crate::test_support::WritableTestBackend;
use machine::{gib_tenths, mib_tenths, ms, system, Moment};

/// One of the sketch's processes.
struct Sketched {
    pid: u32,
    name: &'static str,
    command: &'static str,
    owner: &'static str,
    state: ProcessState,
    nice: i32,
    threads: u32,
    /// CPU time at the later sample, and how much of it since the first.
    time: (Duration, Duration),
    /// VIRT and RES.
    memory: (MemoryBytes, MemoryBytes),
}

impl Sketched {
    fn sample(&self, moment: Moment) -> ProcessSample {
        let (time, since) = self.time;
        ProcessSample {
            name: self.name.to_owned(),
            command: self.command.to_owned(),
            owner: Owner::Named(self.owner.to_owned()),
            state: self.state,
            priority: Priority::Unix {
                kernel: KernelPriority::new(20 + self.nice),
                nice: Niceness::try_from(self.nice).unwrap(),
            },
            threads: self.threads,
            cpu_time: match moment {
                Moment::First => time.checked_sub(since).unwrap(),
                Moment::Later => time,
            },
            memory: Some(MemoryUsage {
                virtual_size: self.memory.0,
                resident: self.memory.1,
                shared: Some(mib_tenths(10)),
            }),
            ..process(pid(self.pid), self.name)
        }
    }
}

/// The sketch's four user processes.
const SKETCHED: [Sketched; 4] = [
    Sketched {
        pid: 4242,
        name: "firefox",
        command: "/usr/lib/firefox/firefox -contentproc -childID 12 -isForBrowser 7 tab",
        owner: "bob",
        state: ProcessState::Running,
        nice: 0,
        threads: 80,
        time: (ms(724_550), ms(480)),
        memory: (gib_tenths(31), mib_tenths(6_120)),
    },
    Sketched {
        pid: 812,
        name: "gnome-shell",
        command: "/usr/bin/gnome-shell",
        owner: "bob",
        state: ProcessState::Sleeping,
        nice: 0,
        threads: 20,
        time: (ms(191_020), ms(75)),
        memory: (gib_tenths(12), mib_tenths(3_010)),
    },
    Sketched {
        pid: 1,
        name: "systemd",
        command: "/sbin/init splash",
        owner: "root",
        state: ProcessState::Sleeping,
        nice: 0,
        threads: 1,
        time: (ms(92_070), ms(0)),
        memory: (mib_tenths(224), mib_tenths(145)),
    },
    Sketched {
        pid: 2001,
        name: "sleep",
        command: "sleep 30",
        owner: "bob",
        state: ProcessState::Sleeping,
        nice: 5,
        threads: 1,
        time: (ms(0), ms(0)),
        memory: (mib_tenths(74), mib_tenths(40)),
    },
];

/// A kernel thread, hidden until `K`.
pub(super) fn kthreadd() -> ProcessSample {
    ProcessSample {
        command: "[kthreadd]".to_owned(),
        owner: Owner::Named("root".to_owned()),
        kind: ProcessKind::KernelThread,
        ..process(pid(2), "kthreadd")
    }
}

/// The sketch's sample at `moment`: every process, and the kernel thread.
fn sketch_sample(moment: Moment) -> Sample {
    let mut processes: Vec<ProcessSample> =
        SKETCHED.iter().map(|each| each.sample(moment)).collect();
    processes.push(kthreadd());
    sample(system(moment), processes)
}

/// The task list of the sketch: two samples apart, sorted by CPU%, with
/// 812 and 2001 marked.
pub(super) fn sketch() -> TaskApp {
    let mut opened = TaskApp::open(Vec::new(), Privilege::User);
    opened.apply(sketch_sample(Moment::First));
    opened.apply(sketch_sample(Moment::Later));
    let list = opened.list();
    for marked in [812, 2001] {
        let at = list
            .view()
            .iter()
            .position(|row| row.key() == key_of(marked));
        list.select_index(at.unwrap(), 1);
        list.toggle_mark(1);
    }
    list.select_index(0, 1);
    opened
}

/// `app` drawn on a `width` × `height` terminal.
pub(super) fn draw(opened: &mut TaskApp, width: u16, height: u16) -> Terminal<WritableTestBackend> {
    let mut terminal = Terminal::new(WritableTestBackend::new(width, height)).unwrap();
    terminal
        .draw(|f| crate::ui::ui(f, &mut opened.app))
        .unwrap();
    terminal
}

/// Each row of `buffer` as text.
pub(super) fn lines(buffer: &Buffer) -> Vec<String> {
    let area = buffer.area;
    (area.y..area.bottom())
        .map(|y| {
            (area.x..area.right())
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}

/// The foreground of the first cell of row `y` that shows `text`.
pub(super) fn fg_of(buffer: &Buffer, y: u16, text: &str) -> Color {
    let row = &lines(buffer)[usize::from(y)];
    let before = row.split_once(text).unwrap().0;
    let x = u16::try_from(before.chars().count()).unwrap();
    buffer[(x, y)].fg
}
