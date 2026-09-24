//! The fixture the task list's screen tests share: the machine and the
//! processes of the design's sketch, and drawing a list to rows of text.

use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::style::Color;
use ratatui::Terminal;

use crate::models::{
    CoreSample, CpuCounters, KernelPriority, Load, LoadAverage, MemoryBytes, MemorySample,
    MemoryUsage, Niceness, Owner, Priority, ProcessKind, ProcessSample, ProcessState, Sample,
    SwapSample, SystemSample,
};
use crate::prompt::Privilege;
use crate::test_support::processes::{process, sample};
use crate::test_support::task_list::{key_of, pid, TaskApp};
use crate::test_support::WritableTestBackend;

/// Mebibytes and gibibytes, as the sizes in the sketch are written.
const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;

/// `tenths` tenths of a gibibyte, in bytes.
const fn gib_tenths(tenths: u64) -> MemoryBytes {
    MemoryBytes::from_bytes(tenths * GIB / 10)
}

/// `tenths` tenths of a mebibyte, in bytes.
const fn mib_tenths(tenths: u64) -> MemoryBytes {
    MemoryBytes::from_bytes(tenths * MIB / 10)
}

const fn ms(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// CPU counters with `user` and `idle` milliseconds.
const fn busy(user: u64, idle: u64) -> CpuCounters {
    CpuCounters {
        user: ms(user),
        nice: Duration::ZERO,
        system: Duration::ZERO,
        idle: ms(idle),
        virtualised: Duration::ZERO,
    }
}

/// Which of the sketch's two samples.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Moment {
    First,
    /// Ten CPU-seconds on: 34.2 % busy in all, each core at its load of
    /// [`CORE_LOADS`].
    Later,
}

/// The sketch's per-core loads, in tenths of a percent.
const CORE_LOADS: [u64; 8] = [120, 81, 403, 30, 95, 22, 179, 14];

/// `counters` at `moment`, `user` and `idle` milliseconds on for the later
/// sample.
fn counters_at(moment: Moment, counters: CpuCounters, user: u64, idle: u64) -> CpuCounters {
    match moment {
        Moment::First => counters,
        Moment::Later => CpuCounters {
            user: counters.user + ms(user),
            idle: counters.idle + ms(idle),
            ..counters
        },
    }
}

/// `kib` kibibytes.
fn kib(kib: u64) -> MemoryBytes {
    MemoryBytes::from_kib(kib).unwrap()
}

/// 125 G of memory, 38.2 G of it used.
fn memory() -> MemorySample {
    let total = 125 * 1024 * 1024;
    let free = total - 40_055_603;
    MemorySample {
        total: kib(total),
        free: kib(free),
        available: kib(free),
        buffers: kib(0),
        cached: kib(0),
        reclaimable: kib(0),
        shared: kib(0),
    }
}

/// The system of the sketch at `moment`: eight cores, the memory of
/// [`memory`], 2 G of swap unused.
fn system(moment: Moment) -> SystemSample {
    let cores = CORE_LOADS
        .iter()
        .zip(0_u16..)
        .map(|(&load, index)| CoreSample {
            index,
            counters: counters_at(moment, busy(1_000, 1_000), load, 1_000 - load),
        });
    let load = |hundredths| Load::from_hundredths(hundredths);
    SystemSample {
        total: counters_at(moment, busy(10_000, 10_000), 3_420, 6_580),
        cores: cores.collect(),
        memory: Some(memory()),
        swap: Some(SwapSample {
            total: kib(2 * 1024 * 1024),
            free: kib(2 * 1024 * 1024),
            cached: kib(0),
        }),
        load: Some(LoadAverage::new(load(52), load(58), load(59))),
        uptime: Some(Duration::from_secs(5 * 3600 + 52 * 60 + 6)),
        running: Some(3),
    }
}

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
