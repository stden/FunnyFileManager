//! Purpose: The robustness corpus: every parser is fed every prefix of every
//! fixture — a file cut short by the reader's cap or a process exiting
//! mid-read — and ten thousand byte flips from a fixed-seed generator — a
//! file hostile or corrupt. No parser may panic: release builds abort on a
//! panic and do not check overflow. And a file read for one value, cut
//! anywhere, reads as nothing or as the whole, never as a smaller number.

use super::parse::{
    command_line, cpu_times, effective_uid, load_average, memory_info, stat, statm, uptime,
    ClockTicks, PageSize,
};
use crate::models::Pid;

/// Byte flips per fixture.
const FLIPS: usize = 10_000;

/// A linear congruential generator (Knuth's MMIX constants): the same
/// sequence on every run and every platform, and no dependency.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    /// A position below `len`, which must not be zero.
    fn below(&mut self, len: usize) -> usize {
        usize::try_from(self.next()).expect("33 bits fit a usize") % len
    }

    fn byte(&mut self) -> u8 {
        self.next().to_le_bytes()[0]
    }
}

/// One file's worth of text and the parser that reads it, answering whether
/// it parsed: the whole fixture must, a damaged one may, and no call may
/// panic.
struct Fixture {
    name: &'static str,
    bytes: &'static [u8],
    parses: fn(&str) -> bool,
}

fn hertz() -> ClockTicks {
    ClockTicks::new(100).expect("not zero")
}

fn page() -> PageSize {
    PageSize::new(4096).expect("not zero")
}

fn pid() -> Pid {
    Pid::new(4242).expect("in range")
}

const FIXTURES: [Fixture; 8] = [
    Fixture {
        name: "stat",
        bytes: b"4242 (next-server (v1) S 812 4242 812 34816 4242 4194560 12345 0 0 0 \
            7245 1210 0 0 20 0 42 0 123456 3321888768 156672 18446744073709551615 1 1 0",
        parses: |text| stat(text, pid(), hertz()).is_some(),
    },
    Fixture {
        name: "statm",
        bytes: b"812345 156672 23456 12 0 345678 0\n",
        parses: |text| statm(text, page()).is_some(),
    },
    Fixture {
        name: "status",
        bytes: b"Name:\tbash\nUmask:\t0022\nState:\tS (sleeping)\nPid:\t812\n\
            Uid:\t1000\t1000\t1000\t1000\nGid:\t1000\t1000\t1000\t1000\n",
        parses: |text| effective_uid(text).is_some(),
    },
    Fixture {
        name: "cmdline",
        bytes: b"python3\0/srv/x/train.py\0--epochs\0\xc3\xa9t\xc3\xa9\0",
        parses: |text| !command_line(text, "python3").starts_with('['),
    },
    Fixture {
        name: "/proc/stat",
        bytes: b"cpu  100 20 30 400 50 6 7 8 9 10\ncpu0 50 10 15 200 25 3 3 4 4 5\n\
            cpu1 50 10 15 200 25 3 4 4 5 5\nintr 123 1 2 3\nctxt 99\n\
            procs_running 3\nprocs_blocked 0\n",
        parses: |text| cpu_times(text, hertz()).is_some(),
    },
    Fixture {
        name: "/proc/meminfo",
        bytes: b"MemTotal:       131636496 kB\nMemFree:        25897000 kB\n\
            MemAvailable:   89695600 kB\nBuffers:         6258456 kB\n\
            Cached:         52449252 kB\nSwapCached:          512 kB\n\
            SReclaimable:    6932888 kB\nShmem:            225196 kB\n\
            SwapTotal:       2097148 kB\nSwapFree:        2096636 kB\n",
        parses: |text| memory_info(text).memory.is_some(),
    },
    Fixture {
        name: "/proc/loadavg",
        bytes: b"0.52 0.58 0.59 3/1204 4242\n",
        parses: |text| load_average(text).is_some(),
    },
    Fixture {
        name: "/proc/uptime",
        bytes: b"21126.06 667890.12\n",
        parses: |text| uptime(text).is_some(),
    },
];

/// Feeds `bytes` to the fixture's parser the way the reader does: decoded
/// lossily.
fn feed(fixture: &Fixture, bytes: &[u8]) -> bool {
    (fixture.parses)(&String::from_utf8_lossy(bytes))
}

#[test]
fn every_whole_fixture_parses() {
    for fixture in &FIXTURES {
        assert!(feed(fixture, fixture.bytes), "{}", fixture.name);
    }
}

#[test]
fn every_prefix_of_every_file_parses_without_a_panic() {
    for fixture in &FIXTURES {
        for end in 0..=fixture.bytes.len() {
            feed(fixture, &fixture.bytes[..end]);
        }
    }
}

/// A file read for one value: its fixture and its parser, the value spelled
/// for comparison.
struct OneValue {
    name: &'static str,
    bytes: &'static [u8],
    value: fn(&str) -> Option<String>,
}

const ONE_VALUE: [OneValue; 4] = [
    OneValue {
        name: "statm",
        bytes: b"812345 156672 23456 12 0 345678 0\n",
        value: |text| statm(text, page()).map(|usage| format!("{usage:?}")),
    },
    OneValue {
        name: "status",
        bytes: b"Name:\tbash\nUid:\t1000\t1001\t1002\t1003\nGid:\t100\t100\t100\t100\n",
        value: |text| effective_uid(text).map(|uid| uid.to_string()),
    },
    OneValue {
        name: "/proc/loadavg",
        bytes: b"10.52 20.58 30.59 3/1204 4242\n",
        value: |text| load_average(text).map(|load| format!("{load:?}")),
    },
    OneValue {
        name: "/proc/uptime",
        bytes: b"21126.06 667890.12\n",
        value: |text| uptime(text).map(|up| format!("{up:?}")),
    },
];

#[test]
fn a_file_cut_short_never_reads_as_a_smaller_number() {
    for file in &ONE_VALUE {
        let whole = (file.value)(&String::from_utf8_lossy(file.bytes));
        assert!(whole.is_some(), "{}", file.name);
        for end in 0..file.bytes.len() {
            let cut = (file.value)(&String::from_utf8_lossy(&file.bytes[..end]));
            assert!(
                cut.is_none() || cut == whole,
                "{} cut after {end} bytes read {cut:?}, not {whole:?}",
                file.name
            );
        }
    }
}

#[test]
fn ten_thousand_byte_flips_per_file_parse_without_a_panic() {
    let mut random = Lcg(0x5eed);
    for fixture in &FIXTURES {
        let mut bytes = fixture.bytes.to_vec();
        for _ in 0..FLIPS {
            let at = random.below(bytes.len());
            bytes[at] = random.byte();
            feed(fixture, &bytes);
            // Now and then start again, so flips pile up and also stay few.
            if random.below(8) == 0 {
                bytes.copy_from_slice(fixture.bytes);
            }
        }
    }
}

#[test]
fn numbers_at_and_past_every_limit_parse_without_a_panic() {
    let limits = [
        "0",
        "1",
        "4294967295",
        "4294967296",
        "18446744073709551615",
        "18446744073709551616",
    ];
    for fixture in &FIXTURES {
        let text = String::from_utf8_lossy(fixture.bytes).into_owned();
        let digits: Vec<&str> = text
            .split(|ch: char| !ch.is_ascii_digit())
            .filter(|run| !run.is_empty())
            .collect();
        for run in digits {
            for limit in limits {
                (fixture.parses)(&text.replacen(run, limit, 1));
            }
        }
    }
}
