//! Purpose: unit tests for the meters a host may not report, load and uptime passed
//! through, and the task counts.

use super::*;

#[test]
fn the_meters_are_absent_without_memory_swap_load_or_uptime() {
    let meters = meters_after(counters(1, 1, 1));
    assert_eq!(meters.memory(), None);
    assert_eq!(meters.swap(), None);
    assert_eq!(meters.load(), None);
    assert_eq!(meters.uptime(), None);
}

#[test]
fn load_and_uptime_pass_through() {
    let mut first = sample(system(CpuCounters::default(), &[]), Vec::new());
    let load = LoadAverage::new(
        Load::from_hundredths(52),
        Load::from_hundredths(58),
        Load::from_hundredths(59),
    );
    first.system.load = Some(load);
    first.system.uptime = Some(Duration::from_secs(21_126));
    let (snapshot, _) = Snapshot::derive(&Baselines::default(), first);
    assert_eq!(snapshot.meters().load(), Some(load));
    assert_eq!(
        snapshot.meters().uptime(),
        Some(Duration::from_secs(21_126))
    );
}

fn with_threads(raw: u32, threads: u32, kind: ProcessKind) -> ProcessSample {
    ProcessSample {
        threads,
        kind,
        ..process(raw, "task")
    }
}

#[test]
fn tasks_count_processes_extra_threads_and_kernel_threads() {
    let processes = vec![
        with_threads(1, 1, ProcessKind::User),
        with_threads(2, 4, ProcessKind::User),
        with_threads(3, 0, ProcessKind::User),
        with_threads(4, 1, ProcessKind::KernelThread),
        with_threads(5, 1, ProcessKind::KernelThread),
    ];
    let mut first = sample(system(CpuCounters::default(), &[]), processes);
    first.system.running = Some(3);
    let (snapshot, _) = Snapshot::derive(&Baselines::default(), first);
    let tasks = snapshot.meters().tasks();
    let counted = [tasks.processes(), tasks.threads(), tasks.kernel_threads()];
    assert_eq!(counted, [3, 2, 2]);
    assert_eq!(tasks.running(), Some(3));
    assert_eq!(meters_after(counters(1, 1, 1)).tasks().running(), None);
}
