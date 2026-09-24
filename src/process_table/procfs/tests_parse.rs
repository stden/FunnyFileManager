//! Purpose: Tests for the per-process `/proc` parsers: `stat` names with
//! spaces and parentheses, every field it is read for, truncation after every
//! field, a PID from another directory, the nice bounds, and `statm`,
//! `status` and `cmdline`.

use std::time::Duration;

use super::parse::{command_line, effective_uid, stat, statm, ClockTicks, PageSize, Stat};
use crate::models::{
    KernelPriority, MemoryBytes, MemoryUsage, Niceness, Pid, Priority, ProcessKind, ProcessState,
    StartTime,
};

/// Every field of a live `stat` line after the name (fields 3 to 52): a
/// sleeping user process, parent 812, flags 0x400100, 72.45 s user and
/// 12.10 s system time, priority 20, nice 0, 42 threads, started at tick
/// 123456.
const FIELDS: &str = "S 812 4242 812 34816 4242 4194560 12345 0 0 0 7245 1210 0 0 20 0 42 0 \
    123456 3321888768 156672 18446744073709551615 1 1 0 0 0 0 0 4096 17663 0 0 0 17 5 0 0 0 \
    0 0 0 0 0 0 0 0 0 0";

fn hertz() -> ClockTicks {
    ClockTicks::new(100).expect("USER_HZ is not zero")
}

fn pid(raw: u32) -> Pid {
    Pid::new(raw).expect("a fixture PID")
}

/// A `stat` line of `pid` named `name`, with the fields numbered in `with`
/// (proc(5) numbering) replaced.
fn stat_line(pid: &str, name: &str, with: &[(usize, &str)]) -> String {
    let mut fields: Vec<&str> = FIELDS.split_ascii_whitespace().collect();
    for &(number, value) in with {
        fields[number - 3] = value;
    }
    format!("{pid} ({name}) {}", fields.join(" "))
}

fn firefox(with: &[(usize, &str)]) -> Option<Stat> {
    stat(&stat_line("4242", "firefox", with), pid(4242), hertz())
}

#[test]
fn a_stat_line_gives_every_field_the_table_reads() {
    let expected = Stat {
        name: "firefox".to_owned(),
        state: ProcessState::Sleeping,
        parent: Some(pid(812)),
        kind: ProcessKind::User,
        cpu_time: Duration::from_millis(84_550),
        priority: Priority::Unix {
            kernel: KernelPriority::new(20),
            nice: Niceness::try_from(0).expect("a nice value"),
        },
        threads: 42,
        start: StartTime::new(123_456),
        started: Duration::from_millis(1_234_560),
    };
    assert_eq!(firefox(&[]), Some(expected));
}

#[test]
fn a_name_may_hold_spaces_parentheses_and_a_newline() {
    for name in [
        "(sd-pam)",
        "npm exec vite p",
        "next-server (v1",
        "one\ntwo",
        ") (",
        "",
    ] {
        let line = stat_line("7", name, &[]);
        let parsed = stat(&line, pid(7), hertz()).map(|stat| stat.name);
        assert_eq!(parsed.as_deref(), Some(name), "{line:?}");
    }
}

#[test]
fn a_name_that_is_not_utf8_is_decoded_lossily() {
    let mut bytes = b"7 (".to_vec();
    bytes.extend_from_slice(&[0xff, b'x', 0xfe]);
    bytes.extend_from_slice(format!(") {FIELDS}").as_bytes());
    let text = String::from_utf8_lossy(&bytes);
    let name = stat(&text, pid(7), hertz()).map(|stat| stat.name);
    assert_eq!(name.as_deref(), Some("\u{fffd}x\u{fffd}"));
}

#[test]
fn a_zero_parent_is_none_and_the_kthread_flag_makes_a_kernel_thread() {
    let kthreadd = firefox(&[(4, "0"), (9, "2129984")]).expect("valid");
    assert_eq!(kthreadd.parent, None);
    assert_eq!(kthreadd.kind, ProcessKind::KernelThread);
    let user = firefox(&[(9, "4194560")]).expect("valid");
    assert_eq!(user.kind, ProcessKind::User);
}

#[test]
fn a_line_cut_after_any_field_before_the_twenty_third_is_malformed() {
    let fields: Vec<&str> = FIELDS.split_ascii_whitespace().collect();
    for kept in 0..=fields.len() {
        let line = format!("4242 (firefox) {}", fields[..kept].join(" "));
        let parsed = stat(&line, pid(4242), hertz());
        assert_eq!(parsed.is_some(), kept >= 21, "{kept} fields");
    }
    for cut in ["", "4242", "4242 (", "4242 (firefox", "4242 (firefox)"] {
        assert_eq!(stat(cut, pid(4242), hertz()), None, "{cut:?}");
    }
}

#[test]
fn a_line_about_another_process_is_rejected() {
    let line = stat_line("4243", "firefox", &[]);
    assert_eq!(stat(&line, pid(4242), hertz()), None);
    let above = stat_line("2147483648", "x", &[]);
    assert_eq!(stat(&above, pid(i32::MAX.unsigned_abs()), hertz()), None);
    for sign in ["+4242", "-4242", " 4242", "0x1092"] {
        assert_eq!(
            stat(&stat_line(sign, "x", &[]), pid(4242), hertz()),
            None,
            "{sign}"
        );
    }
}

#[test]
fn nice_values_outside_minus_twenty_to_nineteen_are_malformed() {
    for (nice, valid) in [("-21", false), ("-20", true), ("19", true), ("20", false)] {
        assert_eq!(firefox(&[(19, nice)]).is_some(), valid, "nice {nice}");
    }
    let realtime = firefox(&[(18, "-100")]).expect("valid");
    let kernel = match realtime.priority {
        Priority::Unix { kernel, .. } => kernel,
        other => panic!("a Unix priority, not {other:?}"),
    };
    assert!(kernel.is_realtime());
}

#[test]
fn every_number_is_checked_and_nothing_overflows() {
    let malformed = [
        (3, "SS"),
        (4, "2147483648"),
        (14, "-1"),
        (20, "4294967296"),
        (22, "18446744073709551616"),
        (18, "high"),
    ];
    for (field, value) in malformed {
        assert_eq!(firefox(&[(field, value)]), None, "field {field} = {value}");
    }
    let most = u64::MAX.to_string();
    let huge = firefox(&[(14, &most), (15, &most), (22, &most)]).expect("valid");
    let limit = hertz().duration(u64::MAX);
    assert_eq!((huge.cpu_time, huge.started), (limit, limit));
    let other = firefox(&[(3, "W")]).map(|stat| stat.state.letter());
    assert_eq!(other, Some('W'));
}

#[test]
fn ticks_become_exact_durations() {
    assert_eq!(ClockTicks::new(0), None);
    assert_eq!(hertz().duration(12_345), Duration::from_millis(123_450));
    let thirds = ClockTicks::new(3).expect("not zero");
    assert_eq!(thirds.duration(4), Duration::new(1, 333_333_333));
}

#[test]
fn statm_multiplies_pages_and_refuses_to_overflow() {
    let page = PageSize::new(4096).expect("not zero");
    let expected = MemoryUsage {
        virtual_size: MemoryBytes::from_bytes(4_096_000),
        resident: MemoryBytes::from_bytes(819_200),
        shared: Some(MemoryBytes::from_bytes(204_800)),
    };
    assert_eq!(statm("1000 200 50 10 0 300 0\n", page), Some(expected));
    assert_eq!(statm("18446744073709551615 1 1 0 0 0 0\n", page), None);
    assert_eq!(statm("1000 200\n", page), None);
    assert_eq!(statm("1000 x 50\n", page), None);
    assert_eq!(PageSize::new(0), None);
}

#[test]
fn status_gives_the_effective_uid() {
    let status =
        "Name:\tbash\nUmask:\t0022\nUid:\t1000\t1001\t1002\t1003\nGid:\t100\t100\t100\t100\n";
    assert_eq!(effective_uid(status), Some(1001));
    assert_eq!(effective_uid("Uid:\t1000\n"), None);
    assert_eq!(effective_uid("Name:\tbash\n"), None);
    assert_eq!(effective_uid("Uid:\t1000\tmany\n"), None);
    assert_eq!(
        effective_uid("Uid:\t0\t4000000000\t0\t0\n"),
        Some(4_000_000_000)
    );
}

#[test]
fn a_command_line_joins_its_arguments() {
    let train = "python3\0/srv/x/train.py\0--epochs\0";
    assert_eq!(
        command_line(train, "python3"),
        "python3 /srv/x/train.py --epochs"
    );
    assert_eq!(
        command_line("a\0\0b\0", "x"),
        "a  b",
        "an empty argument stays"
    );
    let rewritten = "nginx: worker process";
    assert_eq!(command_line(rewritten, "nginx"), rewritten);
}

#[test]
fn an_empty_command_line_shows_the_name_in_brackets() {
    for empty in ["", "\0", "\0\0", " "] {
        assert_eq!(command_line(empty, "kthreadd"), "[kthreadd]", "{empty:?}");
    }
}

#[test]
fn a_command_line_cut_mid_character_still_decodes() {
    // The reader stops at a byte count, which can fall inside a character.
    let cut = 4096;
    let long = format!("{}é tail", "x".repeat(cut - 1));
    let read = &long.as_bytes()[..cut];
    let line = command_line(&String::from_utf8_lossy(read), "x");
    assert!(line.ends_with('\u{fffd}'), "the cut character is replaced");
    assert_eq!(line.chars().count(), cut);
}
