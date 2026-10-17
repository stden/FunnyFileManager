//! Purpose: Tests for the system-wide `/proc` parsers besides `meminfo`,
//! whose tests are in `tests_parse_meminfo.rs`: CPU lines of four, eight and
//! ten fields with guest time folded once, cores in order, `procs_running`
//! behind a long `intr` line, and load and uptime to the hundredth.

use std::time::Duration;

use super::parse::{cpu_times, load_average, memory_info, uptime, ClockTicks};
use crate::models::{CpuCounters, Load, LoadAverage};

fn hertz() -> ClockTicks {
    ClockTicks::new(100).expect("USER_HZ is not zero")
}

/// Counters from tick counts at 100 Hz.
fn ticks(user: u64, nice: u64, system: u64, idle: u64, virtualised: u64) -> CpuCounters {
    let at = |count: u64| Duration::from_millis(count * 10);
    CpuCounters {
        user: at(user),
        nice: at(nice),
        system: at(system),
        idle: at(idle),
        virtualised: at(virtualised),
    }
}

fn total(text: &str) -> Option<CpuCounters> {
    cpu_times(text, hertz()).map(|times| times.total)
}

#[test]
fn ten_fields_fold_guest_time_out_of_user_and_nice_time_once() {
    let line = "cpu  100 20 30 400 50 6 7 8 9 10\n";
    assert_eq!(total(line), Some(ticks(91, 10, 43, 450, 27)));
}

#[test]
fn older_kernels_print_four_or_eight_fields_and_the_rest_count_as_zero() {
    assert_eq!(total("cpu 1 2 3 4\n"), Some(ticks(1, 2, 3, 4, 0)));
    assert_eq!(total("cpu 1 2 3 4 5 6 7 8\n"), Some(ticks(1, 2, 16, 9, 8)));
}

#[test]
fn guest_time_larger_than_user_time_saturates_at_zero() {
    assert_eq!(
        total("cpu 1 1 0 0 0 0 0 0 9 9\n"),
        Some(ticks(0, 0, 0, 0, 18))
    );
}

#[test]
fn a_malformed_or_missing_cpu_line_fails_the_sample() {
    for text in [
        "cpu 1 2 3\n",
        "cpu 1 2 x 4\n",
        "cpu 1 -2 3 4\n",
        "cpu0 1 2 3 4\n",
        "cpu 1 2 3 4\ncpu0 1 2\n",
        "cpu 1 2 3 4\ncpu99999 1 2 3 4\n",
        "cpu 18446744073709551616 0 0 0\n",
        "",
    ] {
        assert!(cpu_times(text, hertz()).is_none(), "{text:?}");
    }
}

#[test]
fn cores_are_keyed_by_number_sorted_and_counted_once() {
    let text = "cpu 4 0 0 4\ncpu3 1 0 0 1\ncpu0 1 0 0 1\ncpu3 9 9 9 9\ncpufreq 1\ncpu1 2 0 0 2\n";
    let times = cpu_times(text, hertz()).expect("valid");
    let numbers: Vec<u16> = times.cores.iter().map(|core| core.index).collect();
    assert_eq!(
        numbers,
        [0, 1, 3],
        "core 2 is offline; the second cpu3 is ignored"
    );
    assert_eq!(times.cores[2].counters, ticks(1, 0, 0, 1, 0));
    assert!(cpu_times("cpu 1 2 3 4\n", hertz()).is_some_and(|times| times.cores.is_empty()));
}

#[test]
fn procs_running_is_found_behind_a_very_long_intr_line() {
    let interrupts: Vec<String> = (0..100_000).map(|count: u32| count.to_string()).collect();
    let text = format!(
        "cpu 1 2 3 4\nintr {}\nctxt 99\nprocs_running 3\nprocs_blocked 0\n",
        interrupts.join(" ")
    );
    let times = cpu_times(&text, hertz()).expect("valid");
    assert_eq!(times.running, Some(3));
    assert_eq!(
        cpu_times("cpu 1 2 3 4\nprocs_running x\n", hertz()).map(|t| t.running),
        Some(None)
    );
}

#[test]
fn a_line_the_reader_cut_before_its_newline_is_not_read() {
    // `procs_running 12` and `cpu0 1 2 3 40`, each cut one digit short.
    let cut = cpu_times("cpu 1 2 3 4\ncpu0 1 2 3 4\nprocs_running 1", hertz());
    let running = cut.as_ref().map(|times| times.running);
    assert_eq!(running, Some(None));
    assert_eq!(cut.map(|times| times.cores.len()), Some(1));
    let cores = cpu_times("cpu 1 2 3 4\ncpu0 1 2 3 4", hertz()).map(|times| times.cores);
    assert_eq!(cores, Some(Vec::new()));
    assert_eq!(total("cpu 1 2 3 40"), None, "the aggregate line itself");
    let memory = memory_info("MemFree: 8 kB\nMemTotal: 13163").memory;
    assert_eq!(memory, None, "MemTotal cut short is no total");
}

fn load(one: u32, five: u32, fifteen: u32) -> LoadAverage {
    LoadAverage::new(
        Load::from_hundredths(one),
        Load::from_hundredths(five),
        Load::from_hundredths(fifteen),
    )
}

#[test]
fn loads_are_whole_hundredths_without_floating_point() {
    assert_eq!(
        load_average("0.52 0.58 0.59 3/1204 4242\n"),
        Some(load(52, 58, 59))
    );
    assert_eq!(
        load_average("12.5 1 100.999 1/1 1\n"),
        Some(load(1250, 100, 10_099))
    );
    for malformed in [
        "0.5x 1 1\n",
        "-1.00 1 1\n",
        "1 1\n",
        ".5 1 1\n",
        "42949673 0 0\n",
        "",
    ] {
        assert_eq!(load_average(malformed), None, "{malformed:?}");
    }
}

#[test]
fn uptime_is_read_to_the_hundredth() {
    assert_eq!(
        uptime("21126.06 667890.12\n"),
        Some(Duration::from_millis(21_126_060))
    );
    assert_eq!(uptime("5 1\n"), Some(Duration::from_secs(5)));
    assert_eq!(uptime("1.9 1\n"), Some(Duration::from_millis(1_900)));
    for malformed in ["", "abc\n", "1.2.3\n", "184467440737095516.16\n"] {
        assert_eq!(uptime(malformed), None, "{malformed:?}");
    }
}
