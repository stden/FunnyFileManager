//! Unit tests for the task list's formats, at every unit boundary.

use std::time::Duration;

use super::format::{
    cpu_meter, cpu_time, cpu_usage, load, memory_share, owner, percent, priority, size, span,
    tasks, uptime, used_of, UNKNOWN,
};
use crate::models::{
    CpuMeter, CpuUsage, KernelPriority, Load, LoadAverage, MemoryBytes, Niceness, Owner, Percent,
    Priority, PriorityClass, ProcessKind, ProcessSample, TaskCounts,
};
use crate::prompt::Privilege;
use crate::test_support::processes::{counters, one_core, process, sample, system};
use crate::test_support::task_list::{pid, TaskApp};

fn share(hundredths: u32) -> Percent {
    Percent::from_hundredths(hundredths)
}

#[test]
fn a_share_has_one_decimal_below_a_hundred_and_none_from_there() {
    let cases = [
        (0, "0.0"),
        (4, "0.0"),
        (5, "0.1"),
        (3_844, "38.4"),
        (3_845, "38.5"),
        (9_990, "99.9"),
        (9_994, "99.9"),
        (9_995, "100"),
        (10_000, "100"),
        (10_049, "100"),
        (10_050, "101"),
        (320_000, "3200"),
        (u32::MAX, "42949673"),
    ];
    for (hundredths, text) in cases {
        assert_eq!(percent(share(hundredths)), text, "{hundredths}");
    }
}

#[test]
fn an_unmeasured_share_is_a_dash_never_a_made_up_zero() {
    assert_eq!(cpu_usage(CpuUsage::Pending), UNKNOWN);
    assert_eq!(cpu_usage(CpuUsage::Measured(share(3_840))), "38.4");
    assert_eq!(memory_share(None), UNKNOWN);
    assert_eq!(memory_share(Some(share(190))), "1.9");
}

fn at(seconds: u64, millis: u64) -> Duration {
    Duration::from_secs(seconds) + Duration::from_millis(millis)
}

#[test]
fn cpu_time_switches_units_at_an_hour_and_at_a_hundred_hours() {
    let cases = [
        (at(0, 0), "0:00.00"),
        (at(92, 70), "1:32.07"),
        (at(724, 559), "12:04.55"),
        (at(3_599, 999), "59:59.99"),
        (at(3_600, 0), "1h00:00"),
        (at(43_495, 990), "12h04:55"),
        (at(359_999, 999), "99h59:59"),
        (at(360_000, 0), "4d04h"),
        (at(363_600, 0), "4d05h"),
        (Duration::MAX, "213503982334601d07h"),
    ];
    for (time, text) in cases {
        assert_eq!(cpu_time(time), text, "{time:?}");
    }
}

#[test]
fn uptime_counts_days_in_words_and_the_rest_on_a_clock() {
    let clock = 5 * 3_600 + 52 * 60 + 6;
    assert_eq!(uptime(Duration::from_secs(clock)), "Uptime: 05:52:06");
    assert_eq!(
        uptime(Duration::from_secs(86_400 + clock)),
        "Uptime: 1 day, 05:52:06"
    );
    assert_eq!(
        uptime(Duration::from_secs(12 * 86_400 + clock)),
        "Uptime: 12 days, 05:52:06"
    );
    assert_eq!(span(Duration::from_secs(86_399)), "23:59:59");
    assert_eq!(span(Duration::from_hours(24)), "1 day, 00:00:00");
}

#[test]
fn a_load_average_is_written_to_the_hundredth() {
    let average = LoadAverage::new(
        Load::from_hundredths(52),
        Load::from_hundredths(5),
        Load::from_hundredths(1_259),
    );
    assert_eq!(load(average), "Load average: 0.52 0.05 12.59");
}

/// The task counts of a sample of `processes`, `running` runnable.
fn counts_of(processes: Vec<ProcessSample>, running: Option<u32>) -> TaskCounts {
    let mut system = system(counters(1, 1, 1), &[counters(1, 1, 1)]);
    system.running = running;
    let mut opened = TaskApp::open(Vec::new(), Privilege::User);
    opened.apply(sample(system, processes));
    opened.list().snapshot().unwrap().meters().tasks()
}

#[test]
fn the_tasks_field_leaves_out_the_runnable_count_it_does_not_have() {
    let threaded = ProcessSample {
        threads: 5,
        ..process(pid(10), "worker")
    };
    let kernel = ProcessSample {
        kind: ProcessKind::KernelThread,
        ..process(pid(2), "kthreadd")
    };
    let processes = vec![threaded, process(pid(11), "sh"), kernel];
    assert_eq!(
        tasks(counts_of(processes.clone(), Some(3))),
        "Tasks: 2, 4 thr, 1 kthr; 3 running"
    );
    assert_eq!(tasks(counts_of(processes, None)), "Tasks: 2, 4 thr, 1 kthr");
}

#[test]
fn sizes_are_written_as_the_panels_write_them() {
    let gib = 1024 * 1024 * 1024;
    assert_eq!(size(None), UNKNOWN);
    assert_eq!(size(Some(MemoryBytes::from_bytes(381 * 1024))), "381 K");
    assert_eq!(
        used_of(MemoryBytes::from_bytes(0), MemoryBytes::from_bytes(2 * gib)),
        "0/2.0 G"
    );
}

#[test]
fn a_cpu_meter_is_captioned_with_its_busy_share() {
    assert_eq!(cpu_meter(CpuMeter::Pending), UNKNOWN);
    let mut opened = TaskApp::open(Vec::new(), Privilege::User);
    opened.apply(one_core(Vec::new()));
    let later = system(counters(2, 1, 3), &[counters(2, 1, 3)]);
    opened.apply(sample(later, Vec::new()));
    let meter = opened.list().snapshot().unwrap().meters().cpu();
    assert_eq!(cpu_meter(meter), "33.3%");
}

#[test]
fn priority_shows_rt_for_realtime_and_dashes_where_there_is_no_nice() {
    let unix = |kernel, nice| Priority::Unix {
        kernel: KernelPriority::new(kernel),
        nice: Niceness::try_from(nice).unwrap(),
    };
    assert_eq!(priority(unix(20, 0)), ("20".to_owned(), "0".to_owned()));
    assert_eq!(priority(unix(39, 19)), ("39".to_owned(), "19".to_owned()));
    assert_eq!(
        priority(unix(-100, -20)),
        ("RT".to_owned(), "-20".to_owned())
    );
    assert_eq!(priority(unix(-99, 0)), ("-99".to_owned(), "0".to_owned()));
    let dashes = (UNKNOWN.to_owned(), UNKNOWN.to_owned());
    assert_eq!(priority(Priority::Unknown), dashes);
    let windows = Priority::Windows {
        class: PriorityClass::Normal,
    };
    assert_eq!(priority(windows), dashes);
}

#[test]
fn an_owner_is_its_printable_name_its_uid_or_a_dash() {
    assert_eq!(owner(&Owner::Named("bob".to_owned())), "bob");
    assert_eq!(owner(&Owner::Named("b\u{7}ob".to_owned())), "b\u{FFFD}ob");
    assert_eq!(owner(&Owner::Uid(4_000_000_000)), "4000000000");
    assert_eq!(owner(&Owner::Unknown), UNKNOWN);
}
